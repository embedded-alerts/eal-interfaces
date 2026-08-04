        create extension if not exists pgcrypto;

        create table if not exists alert_rules (
            id uuid primary key default gen_random_uuid(),
            title text not null check (length(title) between 1 and 256),
            summary text not null default '' check (length(summary) <= 4000),
            query text not null,
    threshold real not null,
    delivery_channel text not null,
    enabled boolean not null,
            status text not null default 'draft',
            created_at timestamptz not null default now(),
            updated_at timestamptz not null default now()
        );

        create index if not exists alert_rules_status_created_idx
          on alert_rules(status, created_at desc, id);

        alter table alert_rules enable row level security;

        -- Production must replace this deny-by-default baseline with explicit
        -- tenant-scoped policies tied to authenticated subjects.
        drop policy if exists deny_anon_alert_rules on alert_rules;
        create policy deny_anon_alert_rules on alert_rules
          for all to anon using (false) with check (false);

        create extension if not exists vector;

create table if not exists alert_documents (
    id uuid primary key default gen_random_uuid(),
    source_uri text not null,
    content_sha256 text not null unique,
    content_text text not null,
    embedding vector(1536),
    published_at timestamptz,
    ingested_at timestamptz not null default now()
);

create index if not exists alert_documents_embedding_hnsw
  on alert_documents using hnsw (embedding vector_cosine_ops);
