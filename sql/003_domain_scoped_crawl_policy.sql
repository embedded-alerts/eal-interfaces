-- Typed public-page source policy and durable crawl leasing for semantic contract v2.
--
-- External search providers, feeds, and sitemaps may discover candidate URLs, but a
-- candidate is never authoritative. Workers must revalidate this exact policy after
-- every redirect, resolve DNS immediately before each request, obey robots.txt, fetch
-- bounded content, and create a local source revision before semantic matching.

create or replace function eal_valid_public_hosts(hosts text[])
returns boolean
language sql
immutable
parallel safe
as $$
    select cardinality(hosts) between 1 and 64
       and coalesce(
            (
                select bool_and(
                    host = lower(host)
                    and host ~ '^[a-z0-9][a-z0-9.-]*\.[a-z]{2,63}$'
                    and host !~ '(^|\.)localhost$'
                    and host !~ '(^|\.)([0-9]{1,3}\.){3}[0-9]{1,3}$'
                    and position('..' in host) = 0
                    and position('*' in host) = 0
                    and position('/' in host) = 0
                    and position(':' in host) = 0
                    and position('@' in host) = 0
                )
                from unnest(hosts) as value(host)
            ),
            false
       )
$$;

create or replace function eal_valid_path_prefixes(prefixes text[])
returns boolean
language sql
immutable
parallel safe
as $$
    select cardinality(prefixes) between 1 and 128
       and coalesce(
            (
                select bool_and(
                    left(prefix, 1) = '/'
                    and position('?' in prefix) = 0
                    and position('#' in prefix) = 0
                    and prefix !~ '(^|/)\.\.(/|$)'
                )
                from unnest(prefixes) as value(prefix)
            ),
            false
       )
$$;

create or replace function eal_valid_content_types(content_types text[])
returns boolean
language sql
immutable
parallel safe
as $$
    select cardinality(content_types) between 1 and 32
       and coalesce(
            (
                select bool_and(
                    content_type = lower(content_type)
                    and content_type ~ '^[a-z0-9!#$&^_.+-]+/[a-z0-9!#$&^_.+-]+$'
                )
                from unnest(content_types) as value(content_type)
            ),
            false
       )
$$;

alter table eal_sources
    add column if not exists allowed_hosts text[] not null default '{}';
alter table eal_sources
    add column if not exists allowed_path_prefixes text[] not null default array['/']::text[];
alter table eal_sources
    add column if not exists include_subdomains boolean not null default false;
alter table eal_sources
    add column if not exists discovery_modes text[] not null default '{}';
alter table eal_sources
    add column if not exists max_depth smallint not null default 3;
alter table eal_sources
    add column if not exists max_pages_per_run integer not null default 1000;
alter table eal_sources
    add column if not exists max_concurrent_requests_per_host smallint not null default 2;
alter table eal_sources
    add column if not exists request_timeout_seconds smallint not null default 20;
alter table eal_sources
    add column if not exists max_response_bytes bigint not null default 5000000;
alter table eal_sources
    add column if not exists allowed_content_types text[] not null default array[
        'text/html',
        'application/xhtml+xml',
        'application/rss+xml',
        'application/atom+xml',
        'application/feed+json'
    ]::text[];
alter table eal_sources
    add column if not exists obey_robots boolean not null default true;

-- Existing pre-contract rows fail closed to their own canonical source host. Search
-- sources must be reviewed before operators broaden the target-domain allowlist.
update eal_sources
set allowed_hosts = array[
        lower(
            regexp_replace(
                split_part(split_part(canonical_url, '://', 2), '/', 1),
                ':[0-9]+$',
                ''
            )
        )
    ]
where kind <> 'api'
  and cardinality(allowed_hosts) = 0;

update eal_sources
set discovery_modes = case kind
        when 'rss' then array['rss']
        when 'atom' then array['atom']
        when 'json_feed' then array['json_feed']
        when 'search_query' then array['search_provider_candidates']
        when 'sitemap' then array['sitemap']
        when 'web_page' then array['manual', 'link']
        else array[]::text[]
    end
where cardinality(discovery_modes) = 0;

alter table eal_sources
    drop constraint if exists eal_sources_public_page_policy_required;
alter table eal_sources
    add constraint eal_sources_public_page_policy_required
    check (
        kind = 'api'
        or (
            eal_valid_public_hosts(allowed_hosts)
            and eal_valid_path_prefixes(allowed_path_prefixes)
            and cardinality(discovery_modes) between 1 and 7
            and discovery_modes <@ array[
                'manual',
                'sitemap',
                'rss',
                'atom',
                'json_feed',
                'search_provider_candidates',
                'link'
            ]::text[]
        )
    );

alter table eal_sources
    drop constraint if exists eal_sources_crawl_budget_valid;
alter table eal_sources
    add constraint eal_sources_crawl_budget_valid
    check (
        max_depth between 0 and 16
        and max_pages_per_run between 1 and 10000
        and max_concurrent_requests_per_host between 1 and 8
        and request_timeout_seconds between 1 and 60
        and max_response_bytes between 1024 and 20000000
        and eal_valid_content_types(allowed_content_types)
    );

alter table eal_sources
    drop constraint if exists eal_sources_public_robots_required;
alter table eal_sources
    add constraint eal_sources_public_robots_required
    check (kind = 'api' or obey_robots);

create unique index if not exists eal_sources_tenant_id_id_uidx
    on eal_sources (tenant_id, id);

create table if not exists eal_crawl_queue (
    id uuid primary key default gen_random_uuid(),
    tenant_id uuid not null references eal_tenants(id) on delete cascade,
    source_id uuid not null,
    candidate_url text not null check (
        length(candidate_url) between 8 and 8192
        and candidate_url ~ '^https?://'
    ),
    canonical_url text not null check (
        length(canonical_url) between 8 and 8192
        and canonical_url ~ '^https?://'
    ),
    discovered_by text not null check (
        discovered_by in (
            'manual',
            'sitemap',
            'rss',
            'atom',
            'json_feed',
            'search_provider_candidates',
            'link'
        )
    ),
    depth smallint not null default 0 check (depth between 0 and 16),
    status text not null default 'pending' check (
        status in (
            'pending',
            'leased',
            'fetched',
            'unchanged',
            'blocked',
            'failed',
            'dead_letter'
        )
    ),
    priority integer not null default 100,
    next_attempt_at timestamptz not null default now(),
    lease_owner text check (lease_owner is null or length(lease_owner) between 1 and 256),
    lease_expires_at timestamptz,
    attempt_count integer not null default 0 check (attempt_count >= 0),
    last_error_class text check (
        last_error_class is null or length(last_error_class) between 1 and 256
    ),
    last_error_detail text,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    foreign key (tenant_id, source_id)
        references eal_sources(tenant_id, id)
        on delete cascade,
    unique (tenant_id, source_id, canonical_url),
    check (
        (
            status = 'leased'
            and lease_owner is not null
            and lease_expires_at is not null
        )
        or (
            status <> 'leased'
            and lease_owner is null
            and lease_expires_at is null
        )
    )
);

create index if not exists eal_crawl_queue_claim_idx
    on eal_crawl_queue (
        tenant_id,
        status,
        next_attempt_at,
        priority,
        created_at,
        id
    )
    where status in ('pending', 'failed');

create index if not exists eal_crawl_queue_expired_lease_idx
    on eal_crawl_queue (tenant_id, lease_expires_at, id)
    where status = 'leased';

alter table eal_crawl_queue enable row level security;
alter table eal_crawl_queue force row level security;
drop policy if exists eal_tenant_isolation on eal_crawl_queue;
create policy eal_tenant_isolation on eal_crawl_queue
    using (tenant_id = eal_current_tenant_id())
    with check (tenant_id = eal_current_tenant_id());

comment on table eal_crawl_queue is
    'Durable, tenant-scoped candidate queue. Discovery never bypasses source policy, fetch validation, revision identity, or local semantic matching.';
