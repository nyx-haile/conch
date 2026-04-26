-- Conch free-trial accounting for Supabase Auth + Postgres.
-- Apply with: supabase db push
-- Server code calls these RPC functions with SUPABASE_SERVICE_ROLE_KEY only.

create extension if not exists pgcrypto;

create table if not exists public.conch_trial_accounts (
  user_id uuid primary key references auth.users(id) on delete cascade,
  email text not null,
  used_cents integer not null default 0 check (used_cents >= 0),
  budget_cents integer not null default 1000 check (budget_cents >= 0),
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create table if not exists public.conch_trial_global_usage (
  singleton boolean primary key default true check (singleton),
  used_cents integer not null default 0 check (used_cents >= 0),
  budget_cents integer not null default 1000000 check (budget_cents >= 0),
  updated_at timestamptz not null default now()
);

create table if not exists public.conch_trial_usage_events (
  id uuid primary key default gen_random_uuid(),
  user_id uuid not null references auth.users(id) on delete cascade,
  event_type text not null,
  amount_cents integer not null check (amount_cents > 0),
  created_at timestamptz not null default now(),
  metadata jsonb not null default '{}'::jsonb
);

alter table public.conch_trial_accounts enable row level security;
alter table public.conch_trial_global_usage enable row level security;
alter table public.conch_trial_usage_events enable row level security;

create or replace function public.conch_ensure_trial_account(
  p_user_id uuid,
  p_email text,
  p_per_user_budget_cents integer default 1000,
  p_global_budget_cents integer default 1000000
)
returns jsonb
language plpgsql
security definer
set search_path = public
as $$
begin
  insert into public.conch_trial_global_usage (singleton, budget_cents)
  values (true, p_global_budget_cents)
  on conflict (singleton) do update
    set budget_cents = excluded.budget_cents,
        updated_at = now();

  insert into public.conch_trial_accounts (user_id, email, budget_cents)
  values (p_user_id, lower(trim(p_email)), p_per_user_budget_cents)
  on conflict (user_id) do update
    set email = excluded.email,
        budget_cents = excluded.budget_cents,
        updated_at = now();

  return jsonb_build_object(
    'ok', true,
    'per_user_budget_cents', p_per_user_budget_cents,
    'global_budget_cents', p_global_budget_cents
  );
end;
$$;

create or replace function public.conch_reserve_trial_usage(
  p_user_id uuid,
  p_email text,
  p_reservation_cents integer,
  p_per_user_budget_cents integer default 1000,
  p_global_budget_cents integer default 1000000
)
returns jsonb
language plpgsql
security definer
set search_path = public
as $$
declare
  v_user_used integer;
  v_user_budget integer;
  v_global_used integer;
  v_global_budget integer;
begin
  if p_reservation_cents <= 0 then
    return jsonb_build_object(
      'ok', false,
      'state', 'usage_config_missing',
      'status', 503,
      'message', 'Usage reservation must be positive.'
    );
  end if;

  perform public.conch_ensure_trial_account(
    p_user_id,
    p_email,
    p_per_user_budget_cents,
    p_global_budget_cents
  );

  select used_cents, budget_cents
    into v_global_used, v_global_budget
    from public.conch_trial_global_usage
    where singleton = true
    for update;

  select used_cents, budget_cents
    into v_user_used, v_user_budget
    from public.conch_trial_accounts
    where user_id = p_user_id
    for update;

  if v_user_used + p_reservation_cents > v_user_budget then
    return jsonb_build_object(
      'ok', false,
      'state', 'usage_limit_reached',
      'status', 402,
      'message', 'This free-trial email has reached its $10 managed-usage budget.',
      'per_user_used_cents', v_user_used,
      'per_user_budget_cents', v_user_budget,
      'global_used_cents', v_global_used,
      'global_budget_cents', v_global_budget
    );
  end if;

  if v_global_used + p_reservation_cents > v_global_budget then
    return jsonb_build_object(
      'ok', false,
      'state', 'free_trials_closed',
      'status', 403,
      'message', 'Free trials are temporarily closed after the managed-usage pool reached $10,000.',
      'per_user_used_cents', v_user_used,
      'per_user_budget_cents', v_user_budget,
      'global_used_cents', v_global_used,
      'global_budget_cents', v_global_budget
    );
  end if;

  update public.conch_trial_accounts
    set used_cents = used_cents + p_reservation_cents,
        updated_at = now()
    where user_id = p_user_id
    returning used_cents into v_user_used;

  update public.conch_trial_global_usage
    set used_cents = used_cents + p_reservation_cents,
        updated_at = now()
    where singleton = true
    returning used_cents into v_global_used;

  insert into public.conch_trial_usage_events (user_id, event_type, amount_cents, metadata)
  values (p_user_id, 'deepgram_token_grant_reserved', p_reservation_cents, jsonb_build_object('source', 'vercel_api'));

  return jsonb_build_object(
    'ok', true,
    'reservation_cents', p_reservation_cents,
    'per_user_used_cents', v_user_used,
    'per_user_budget_cents', v_user_budget,
    'global_used_cents', v_global_used,
    'global_budget_cents', v_global_budget
  );
end;
$$;

revoke all on function public.conch_ensure_trial_account(uuid, text, integer, integer) from public;
revoke all on function public.conch_reserve_trial_usage(uuid, text, integer, integer, integer) from public;
grant execute on function public.conch_ensure_trial_account(uuid, text, integer, integer) to service_role;
grant execute on function public.conch_reserve_trial_usage(uuid, text, integer, integer, integer) to service_role;
