drop function if exists check_account_default_profile;
drop trigger if exists tcheck_account_default_profile;

drop table if exists profile;
drop table if exists account;

drop domain if exists email;
drop domain if exists username;

drop extension pg_uuidv7 if exists;
