-- Embedded Alerts semantic ingestion and delivery model.
--
-- This append-only migration leaves the bootstrap tables from 001_initial.sql intact.
-- New services must use the eal_* tables below. Every tenant-scoped transaction must
-- set `SET LOCAL app.tenant_id = '<uuid>'` after authentication.

create extension if not exists pgcrypto;
create extension if not exists vector;

create or replace function eal_current_tenant_id()
returns uuid
language sql
stable
as $$
    select coalesce(
        nullif(current_setting('app.tenant_id', true), '')::uuid,
        nullif(current_setting('request.jwt.claim.tenant_id', true), '')::uuid,
        nullif(
            nullif(current_setting('request.jwt.claims', true), '')::jsonb ->> 'tenant_id',
            ''
        )::uuid
    )
$$;

create table if not exists eal_tenants (
    id uuid primary key default gen_random_uuid(),
    slug text not null unique check (slug ~ '^[a-z0-9][a-z0-9-]{1,62}$'),
    display_name text not null check (length(display_name) between 1 and 256),
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now()
);

create table if not exists eal_user_profiles (
    id uuid primary key default gen_random_uuid(),
    tenant_id uuid not null references eal_tenants(id) on delete cascade,
    external_subject text not null check (length(external_subject) between 1 and 512),
    display_name text not null default '' check (length(display_name) <= 256),
    email text,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    unique (tenant_id, external_subject)
);

create table if not exists eal_embedding_spaces (
    id uuid primary key default gen_random_uuid(),
    provider text not null check (length(provider) between 1 and 128),
    model text not null check (length(model) between 1 and 256),
    model_version text not null check (length(model_version) between 1 and 256),
    dimensions integer not null check (dimensions between 1 and 32768),
    normalization text not null check (normalization in ('none', 'l2')),
    created_at timestamptz not null default now(),
    unique (provider, model, model_version, dimensions, normalization)
);

create table if not exists eal_sources (
    id uuid primary key default gen_random_uuid(),
    tenant_id uuid not null references eal_tenants(id) on delete cascade,
    created_by uuid not null references eal_user_profiles(id) on delete restrict,
    kind text not null check (
        kind in ('rss', 'atom', 'json_feed', 'search_query', 'sitemap', 'web_page', 'api')
    ),
    name text not null check (length(name) between 1 and 256),
    canonical_url text not null check (
        length(canonical_url) between 8 and 8192
        and canonical_url ~ '^https?://'
    ),
    poll_interval_seconds integer not null default 900
        check (poll_interval_seconds between 60 and 604800),
    status text not null default 'active'
        check (status in ('active', 'paused', 'error', 'disabled')),
    enabled boolean not null default true,
    etag text,
    last_modified text,
    last_fetched_at timestamptz,
    next_fetch_at timestamptz not null default now(),
    consecutive_failures integer not null default 0 check (consecutive_failures >= 0),
    last_error_class text,
    last_error_at timestamptz,
    public_config jsonb not null default '{}'::jsonb,
    identity_key text generated always as (
        encode(
            digest(
                concat_ws('|', kind, lower(canonical_url), public_config::text),
                'sha256'
            ),
            'hex'
        )
    ) stored,
    credential_reference text check (
        credential_reference is null or length(credential_reference) between 1 and 2048
    ),
    fetch_policy jsonb not null default '{}'::jsonb,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    unique (tenant_id, identity_key)
);

create table if not exists eal_source_documents (
    id uuid primary key default gen_random_uuid(),
    tenant_id uuid not null references eal_tenants(id) on delete cascade,
    canonical_url text not null check (
        length(canonical_url) between 8 and 8192
        and canonical_url ~ '^https?://'
    ),
    external_id text,
    title text check (title is null or length(title) <= 2048),
    current_revision_id uuid,
    first_seen_at timestamptz not null default now(),
    last_seen_at timestamptz not null default now(),
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    unique (tenant_id, canonical_url)
);

create table if not exists eal_source_document_sources (
    tenant_id uuid not null references eal_tenants(id) on delete cascade,
    source_id uuid not null references eal_sources(id) on delete cascade,
    document_id uuid not null references eal_source_documents(id) on delete cascade,
    first_seen_at timestamptz not null default now(),
    last_seen_at timestamptz not null default now(),
    primary key (source_id, document_id)
);

create table if not exists eal_source_revisions (
    id uuid primary key default gen_random_uuid(),
    tenant_id uuid not null references eal_tenants(id) on delete cascade,
    document_id uuid not null references eal_source_documents(id) on delete cascade,
    previous_revision_id uuid references eal_source_revisions(id) on delete set null,
    content_sha256 text not null check (content_sha256 ~ '^[0-9a-f]{64}$'),
    content_text text not null check (length(content_text) between 1 and 10000000),
    content_type text check (content_type is null or length(content_type) <= 256),
    language text check (language is null or length(language) <= 64),
    published_at timestamptz,
    fetched_at timestamptz not null default now(),
    response_metadata jsonb not null default '{}'::jsonb,
    created_at timestamptz not null default now(),
    unique (document_id, content_sha256)
);

alter table eal_source_documents
    drop constraint if exists eal_source_documents_current_revision_id_fkey;
alter table eal_source_documents
    add constraint eal_source_documents_current_revision_id_fkey
    foreign key (current_revision_id)
    references eal_source_revisions(id)
    on delete set null;

create table if not exists eal_embeddings (
    id uuid primary key default gen_random_uuid(),
    tenant_id uuid not null references eal_tenants(id) on delete cascade,
    source_revision_id uuid not null references eal_source_revisions(id) on delete cascade,
    embedding_space_id uuid not null references eal_embedding_spaces(id) on delete restrict,
    embedding vector not null,
    generated_at timestamptz not null default now(),
    created_at timestamptz not null default now(),
    unique (source_revision_id, embedding_space_id)
);

create or replace function eal_validate_embedding_dimensions()
returns trigger
language plpgsql
as $$
declare
    expected_dimensions integer;
begin
    select dimensions
      into expected_dimensions
      from eal_embedding_spaces
     where id = new.embedding_space_id;

    if expected_dimensions is null then
        raise exception 'unknown embedding space %', new.embedding_space_id;
    end if;

    if vector_dims(new.embedding) <> expected_dimensions then
        raise exception
            'embedding dimension mismatch: got %, expected % for space %',
            vector_dims(new.embedding),
            expected_dimensions,
            new.embedding_space_id;
    end if;

    return new;
end
$$;

drop trigger if exists eal_embeddings_dimension_guard on eal_embeddings;
create trigger eal_embeddings_dimension_guard
before insert or update of embedding, embedding_space_id
on eal_embeddings
for each row
execute function eal_validate_embedding_dimensions();

create table if not exists eal_delivery_targets (
    id uuid primary key default gen_random_uuid(),
    tenant_id uuid not null references eal_tenants(id) on delete cascade,
    user_id uuid not null references eal_user_profiles(id) on delete cascade,
    kind text not null check (kind in ('webhook', 'email', 'slack', 'discord', 'in_app')),
    label text not null check (length(label) between 1 and 256),
    config_reference text not null check (length(config_reference) between 1 and 2048),
    public_metadata jsonb not null default '{}'::jsonb,
    enabled boolean not null default true,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now()
);

create table if not exists eal_alert_rules (
    id uuid primary key default gen_random_uuid(),
    tenant_id uuid not null references eal_tenants(id) on delete cascade,
    owner_user_id uuid not null references eal_user_profiles(id) on delete cascade,
    title text not null check (length(title) between 1 and 256),
    summary text not null default '' check (length(summary) <= 4000),
    status text not null default 'draft'
        check (status in ('draft', 'active', 'paused', 'archived')),
    enabled boolean not null default true,
    current_revision integer not null default 1 check (current_revision >= 1),
    current_revision_id uuid,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now()
);

create table if not exists eal_alert_rule_revisions (
    id uuid primary key default gen_random_uuid(),
    tenant_id uuid not null references eal_tenants(id) on delete cascade,
    alert_rule_id uuid not null references eal_alert_rules(id) on delete cascade,
    revision integer not null check (revision >= 1),
    query_text text not null check (length(query_text) between 1 and 16384),
    embedding_space_id uuid references eal_embedding_spaces(id) on delete restrict,
    query_embedding vector,
    semantic_threshold real not null default 0.78
        check (semantic_threshold between 0 and 1),
    semantic_weight real not null default 0.8
        check (semantic_weight between 0 and 1),
    lexical_weight real not null default 0.2
        check (lexical_weight between 0 and 1),
    required_terms text[] not null default '{}',
    excluded_terms text[] not null default '{}',
    source_ids uuid[] not null default '{}',
    cooldown_seconds integer not null default 3600
        check (cooldown_seconds between 0 and 2592000),
    created_by uuid not null references eal_user_profiles(id) on delete restrict,
    created_at timestamptz not null default now(),
    unique (alert_rule_id, revision),
    check (abs((semantic_weight + lexical_weight) - 1.0) <= 0.0001),
    check ((embedding_space_id is null) = (query_embedding is null))
);

alter table eal_alert_rules
    drop constraint if exists eal_alert_rules_current_revision_id_fkey;
alter table eal_alert_rules
    add constraint eal_alert_rules_current_revision_id_fkey
    foreign key (current_revision_id)
    references eal_alert_rule_revisions(id)
    on delete restrict;

create or replace function eal_validate_rule_embedding_dimensions()
returns trigger
language plpgsql
as $$
declare
    expected_dimensions integer;
begin
    if new.query_embedding is null then
        return new;
    end if;

    select dimensions
      into expected_dimensions
      from eal_embedding_spaces
     where id = new.embedding_space_id;

    if expected_dimensions is null then
        raise exception 'unknown embedding space %', new.embedding_space_id;
    end if;

    if vector_dims(new.query_embedding) <> expected_dimensions then
        raise exception
            'query embedding dimension mismatch: got %, expected % for space %',
            vector_dims(new.query_embedding),
            expected_dimensions,
            new.embedding_space_id;
    end if;

    return new;
end
$$;

drop trigger if exists eal_rule_revisions_dimension_guard on eal_alert_rule_revisions;
create trigger eal_rule_revisions_dimension_guard
before insert or update of query_embedding, embedding_space_id
on eal_alert_rule_revisions
for each row
execute function eal_validate_rule_embedding_dimensions();

create table if not exists eal_rule_delivery_targets (
    tenant_id uuid not null references eal_tenants(id) on delete cascade,
    alert_rule_id uuid not null references eal_alert_rules(id) on delete cascade,
    delivery_target_id uuid not null references eal_delivery_targets(id) on delete cascade,
    created_at timestamptz not null default now(),
    primary key (alert_rule_id, delivery_target_id)
);

create or replace function eal_match_identity(
    p_tenant_id uuid,
    p_rule_revision_id uuid,
    p_source_revision_id uuid,
    p_embedding_space_id uuid,
    p_content_sha256 text
)
returns text
language sql
immutable
as $$
    select encode(
        digest(
            concat_ws(
                '|',
                p_tenant_id::text,
                p_rule_revision_id::text,
                p_source_revision_id::text,
                coalesce(p_embedding_space_id::text, 'lexical-only'),
                lower(p_content_sha256)
            ),
            'sha256'
        ),
        'hex'
    )
$$;

create table if not exists eal_matches (
    id uuid primary key default gen_random_uuid(),
    tenant_id uuid not null references eal_tenants(id) on delete cascade,
    identity_key text not null check (identity_key ~ '^[0-9a-f]{64}$'),
    alert_rule_id uuid not null references eal_alert_rules(id) on delete cascade,
    alert_rule_revision_id uuid not null references eal_alert_rule_revisions(id) on delete restrict,
    source_document_id uuid not null references eal_source_documents(id) on delete cascade,
    source_revision_id uuid not null references eal_source_revisions(id) on delete cascade,
    embedding_space_id uuid references eal_embedding_spaces(id) on delete restrict,
    semantic_score real check (semantic_score is null or semantic_score between 0 and 1),
    lexical_score real not null check (lexical_score between 0 and 1),
    total_score real not null check (total_score between 0 and 1),
    explanation jsonb not null default '{}'::jsonb,
    status text not null default 'candidate'
        check (status in ('candidate', 'suppressed', 'queued', 'delivered', 'dismissed')),
    matched_at timestamptz not null default now(),
    suppressed_until timestamptz,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    unique (tenant_id, identity_key)
);

create table if not exists eal_delivery_attempts (
    id uuid primary key default gen_random_uuid(),
    tenant_id uuid not null references eal_tenants(id) on delete cascade,
    match_id uuid not null references eal_matches(id) on delete cascade,
    delivery_target_id uuid not null references eal_delivery_targets(id) on delete cascade,
    idempotency_key text not null check (length(idempotency_key) between 1 and 512),
    attempt_number integer not null check (attempt_number >= 1),
    status text not null default 'pending'
        check (status in (
            'pending',
            'delivering',
            'succeeded',
            'retry_scheduled',
            'dead_lettered',
            'cancelled'
        )),
    next_attempt_at timestamptz,
    provider_reference text,
    response_status integer check (
        response_status is null or response_status between 100 and 599
    ),
    response_metadata jsonb not null default '{}'::jsonb,
    error_class text,
    error_detail text,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    unique (tenant_id, idempotency_key),
    unique (match_id, delivery_target_id, attempt_number)
);

create table if not exists eal_user_match_state (
    tenant_id uuid not null references eal_tenants(id) on delete cascade,
    user_id uuid not null references eal_user_profiles(id) on delete cascade,
    match_id uuid not null references eal_matches(id) on delete cascade,
    read_at timestamptz,
    dismissed_at timestamptz,
    updated_at timestamptz not null default now(),
    primary key (user_id, match_id)
);

create index if not exists eal_sources_due_idx
    on eal_sources (tenant_id, next_fetch_at, id)
    where enabled and status = 'active';

create index if not exists eal_source_documents_seen_idx
    on eal_source_documents (tenant_id, last_seen_at desc, id);

create index if not exists eal_source_document_sources_source_idx
    on eal_source_document_sources (tenant_id, source_id, last_seen_at desc, document_id);

create index if not exists eal_source_revisions_document_idx
    on eal_source_revisions (tenant_id, document_id, fetched_at desc, id);

create index if not exists eal_embeddings_space_idx
    on eal_embeddings (tenant_id, embedding_space_id, source_revision_id);

comment on table eal_embeddings is
    'Vectors from different embedding spaces must never be compared. Create a partial or partition-local pgvector index per embedding_space_id after choosing an operational model.';

create index if not exists eal_alert_rules_owner_idx
    on eal_alert_rules (tenant_id, owner_user_id, status, updated_at desc, id);

create index if not exists eal_matches_rule_idx
    on eal_matches (tenant_id, alert_rule_id, matched_at desc, id);

create index if not exists eal_matches_delivery_queue_idx
    on eal_matches (tenant_id, status, matched_at, id)
    where status in ('candidate', 'queued');

create index if not exists eal_delivery_attempts_due_idx
    on eal_delivery_attempts (tenant_id, next_attempt_at, id)
    where status in ('pending', 'retry_scheduled');

alter table eal_tenants enable row level security;
alter table eal_tenants force row level security;
drop policy if exists eal_tenant_self on eal_tenants;
create policy eal_tenant_self on eal_tenants
    using (id = eal_current_tenant_id())
    with check (id = eal_current_tenant_id());

do $$
declare
    target_table text;
begin
    foreach target_table in array array[
        'eal_user_profiles',
        'eal_sources',
        'eal_source_documents',
        'eal_source_document_sources',
        'eal_source_revisions',
        'eal_embeddings',
        'eal_delivery_targets',
        'eal_alert_rules',
        'eal_alert_rule_revisions',
        'eal_rule_delivery_targets',
        'eal_matches',
        'eal_delivery_attempts',
        'eal_user_match_state'
    ]
    loop
        execute format('alter table %I enable row level security', target_table);
        execute format('alter table %I force row level security', target_table);
        execute format('drop policy if exists eal_tenant_isolation on %I', target_table);
        execute format(
            'create policy eal_tenant_isolation on %I using (tenant_id = eal_current_tenant_id()) with check (tenant_id = eal_current_tenant_id())',
            target_table
        );
    end loop;
end
$$;
