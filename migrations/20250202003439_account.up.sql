create extension citext;
create extension pg_uuidv7;

-- It *is* actually immutable, and we want to use it for generated columns.
alter function uuid_v7_to_timestamp immutable;

-- citext stores case information, but makes all comparisons (e.g. for unique constraints) case insensitively. So, for example, a user can't register separate accounts as alice@example.com and Alice@example.com.
create domain email as citext
  check ( value ~ '^[a-zA-Z0-9.!#$%&''*+/=?^_`{|}~-]+@[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?(?:\.[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)*$' );

create table account (
  id uuid primary key default uuid_generate_v7(),
  -- 254 characters is the maximum length of an email address per the spec.
  email email unique not null constraint email_not_too_long check (length(email) <= 254),
  secondary_email email[],
  created_at timestamp generated always as (uuid_v7_to_timestamp(id)) stored,
  ask_for_profile_on_login boolean not null default false
);

comment on table account is 'An account (profiled or unprofiled) that is using the site.';
comment on column account.id is 'Account ID.';
comment on column account.email is 'Primary email.';
comment on column account.secondary_email is 'Secondary emails.';
comment on column account.created_at is 'When the account was created.';
comment on column account.ask_for_profile_on_login is 'Setting to ask what profile to log in as every login.';

create domain username as varchar(20)
  check ( value ~ '^[a-z0-9]+$' and substring(value, 1, 1) ~ '[a-z]' );

create table profile (
  id uuid primary key default uuid_generate_v7(),
  username username unique not null constraint username_not_too_short check (length(username) >= 5),
  account_id uuid references account not null,
  display_name varchar(30),
  bio varchar(500),
  created_at timestamp generated always as (uuid_v7_to_timestamp(id)) stored
);

alter table account add column default_profile uuid references profile;
comment on column account.default_profile is 'Default profile. Null is reader mode.';

create or replace function check_account_default_profile() returns trigger as $$
  declare
    profile_not_same_account boolean;
  begin
    if new.default_profile is not null
        and new.default_profile is distinct from old.default_profile then
      select not exists(
        select 1
        from profile
        where id = new.default_profile
          and account_id = new.id
        limit 1
      ) into profile_not_same_account;
      if profile_not_same_account then
        raise exception 'default_profile must exist in the profile table and be owned by the same account';
      end if;
    end if;
    return new;
  end;
$$ language plpgsql;

create constraint trigger tcheck_account_default_profile
  after insert or update on account
  deferrable initially deferred
  for each row execute function check_account_default_profile();

comment on table profile is 'User profile, for non-lurker users.';
comment on column profile.id is 'Profile ID. Unchanging in case the username is changed. Private/transparent to users.';
comment on column profile.username is 'Username.';
comment on column profile.account_id is 'Account association.';
comment on column profile.display_name is 'Display name; shown in UI.';
comment on column profile.bio is 'User bio.';
