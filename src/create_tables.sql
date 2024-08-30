create table if not exists blocks (
  height integer not null,
  hash text primary key not null,
  prev_block_hash text,
  transactions_hash text not null,
  created_at datetime not null,
  consensus_estimation_ms integer not null,
  view_change_index integer
);

create table if not exists domains (
  name text primary key not null,
  logo text,
  metadata json
);

create table if not exists accounts (
  signatory text not null,
  domain text not null references domains(name),
  metadata json,
  primary key (signatory, domain)
);

create table if not exists domain_owners (
  account_signatory text not null,
  account_domain text not null,
  domain text not null references domains(name),
  foreign key (account_signatory, account_domain) references accounts(signatory, domain),
  primary key (account_signatory, account_domain, domain)
);

create table if not exists asset_definitions (
  name text not null,
  domain text not null references domains(name),
  owned_by_signatory text not null,
  owned_by_domain text not null,
  logo text,
  metadata json,
  mintable text check (mintable in ('Once', 'Not', 'Infinitely')) not null,
  type text check (type in ('Numeric', 'Store')) not null,
  primary key (name, domain)
  foreign key (owned_by_signatory, owned_by_domain) references accounts(signatory, domain)
);

create table if not exists assets (
  definition_name text not null,
  definition_domain text not null,
  owned_by_signatory text not null,  
  owned_by_domain text not null,
  value json not null,
  foreign key (definition_name, definition_domain) references asset_definitions(name, domain),
  foreign key (owned_by_signatory, owned_by_domain) references accounts(signatory, domain),
  primary key (definition_name, definition_domain, owned_by_signatory, owned_by_domain)
);

create table if not exists transactions (
  hash text primary key not null,
  block_hash text not null references blocks(hash),
  created_at datetime not null,
  authority_signatory text not null,
  authority_domain text not null,
  signature text not null,
  nonce integer,
  metadata json,
  time_to_live_ms integer,
  instructions text check (instructions in ('Instructions', 'WASM')) not null,
  error json,
  foreign key (authority_signatory, authority_domain) references accounts(signatory, domain)
);

create table if not exists instructions (
  transaction_hash text not null references transactions(hash),
  value json not null
);
