create table if not exists blocks
(
    height            integer primary key not null,
    hash              text                not null,
    prev_block_hash   text,
    transactions_hash text,
    created_at        datetime            not null
);

create table if not exists domains
(
    name     text primary key not null,
    logo     text,
    metadata json
);

create table if not exists accounts
(
    signatory text not null,
    domain    text not null references domains (name),
    metadata  json,
    primary key (signatory, domain)
);

create table if not exists domain_owners
(
    account_signatory text not null,
    account_domain    text not null,
    domain            text not null references domains (name),
    foreign key (account_signatory, account_domain) references accounts (signatory, domain),
    primary key (account_signatory, account_domain, domain)
);

create table if not exists asset_definitions
(
    name               text                                                   not null,
    domain             text                                                   not null references domains (name),
    owned_by_signatory text                                                   not null,
    owned_by_domain    text                                                   not null,
    logo               text,
    metadata           json,
    mintable           text check (mintable in ('Once', 'Not', 'Infinitely')) not null,
    primary key (name, domain),
    foreign key (owned_by_signatory, owned_by_domain) references accounts (signatory, domain)
);


create table if not exists nfts
(
    name               text                                                   not null,
    domain             text                                                   not null references domains (name),
    owned_by_signatory text                                                   not null,
    owned_by_domain    text                                                   not null,
    content            json,
    primary key (name, domain),
    foreign key (owned_by_signatory, owned_by_domain) references accounts (signatory, domain)
    );

create table if not exists assets
(
    definition_name    text not null,
    definition_domain  text not null,
    owned_by_signatory text not null,
    owned_by_domain    text not null,
    value              json not null,
    foreign key (definition_name, definition_domain) references asset_definitions (name, domain),
    foreign key (owned_by_signatory, owned_by_domain) references accounts (signatory, domain),
    primary key (definition_name, definition_domain, owned_by_signatory, owned_by_domain)
);

create table if not exists transactions
(
    hash                text primary key                                    not null,
    block               integer                                             not null references blocks (height),
    created_at          datetime                                            not null,
    authority_signatory text                                                not null,
    authority_domain    text                                                not null,
    signature           text                                                not null,
    nonce               integer,
    metadata            json,
    time_to_live_ms     integer,
    executable          text check (executable in ('Instructions', 'WASM')) not null,
    error               json,
    foreign key (authority_signatory, authority_domain) references accounts (signatory, domain)
);

create table if not exists instructions
(
    transaction_hash text not null references transactions (hash),
    value            json not null
);

/* VIEWS */

create view if not exists v_transactions
as
select *,
       format('%s@%s', authority_signatory, authority_domain)       as authority,
       case when error is null then 'committed' else 'rejected' end as status
from transactions;

create view if not exists v_instructions
as
select json_each.key                                                as kind,
       case
           /* TODO: truncate payload for `Upgrade` instruction kind? */
           when json_each.type in ('null', 'text', 'integer', 'real') then json_quote(json_each.value)
           when json_each.type in ('false', 'true') then json_each.type
           else json_each.value
           end                                                      as payload,
       created_at,
       transaction_hash,
       case when error is null then 'committed' else 'rejected' end as transaction_status,
       format('%s@%s', authority_signatory, authority_domain)       as authority,
       block
from instructions,
     json_each(instructions.value)
         join v_transactions as txs on txs.hash = instructions.transaction_hash;

create view if not exists v_assets as
select case assets.definition_domain = assets.owned_by_domain
           when true then format('%s##%s@%s', assets.definition_name, assets.owned_by_signatory, assets.owned_by_domain)
           else format('%s#%s#%s@%s', assets.definition_name, assets.definition_domain, assets.owned_by_signatory,
                       assets.owned_by_domain) end as id,
       value
from assets;

create view if not exists v_nfts as
select
    *,
    format('%s$%s', nfts.name, nfts.domain) as id,
    format('%s@%s', nfts.owned_by_signatory, nfts.owned_by_domain) as owned_by
from nfts;
