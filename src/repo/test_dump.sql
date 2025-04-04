PRAGMA foreign_keys= OFF;
BEGIN TRANSACTION;
CREATE TABLE blocks
(
    height            integer primary key not null,
    hash              text                not null,
    prev_block_hash   text,
    transactions_hash text,
    created_at        datetime            not null
);
INSERT INTO blocks
VALUES (1, 'dc7fd2f4612ac8698f9828cce22775b1b766db3453a634bbaca8b6a7a18903c7', NULL,
        'a6e55726bff1d1f2f3c35783a8eb2ed99a9bcf9e48b73b2fe812a8d5dcdce1c3', '2025-04-03T03:17:13.162+00:00');
INSERT INTO blocks
VALUES (2, 'd05e5513ccd8e6d163c2801142b19a69fcf16880534b4bac6b4b3410aff8797b',
        'dc7fd2f4612ac8698f9828cce22775b1b766db3453a634bbaca8b6a7a18903c7', NULL, '2025-04-03T03:17:16.011+00:00');
INSERT INTO blocks
VALUES (3, 'd8cb9b6c3637ad9be9aff68606ea21d0b484d2d8025fd1988817316e8b3099bd',
        'd05e5513ccd8e6d163c2801142b19a69fcf16880534b4bac6b4b3410aff8797b',
        '19735c532d67d95971bef55f8a050c39891c9bf13deaf264b4ac072bd0ee3b65', '2025-04-03T03:17:29.318+00:00');
INSERT INTO blocks
VALUES (4, '3e6bbd66992cd9bbe43e847a8ad3192205676658a20f7840a96ca1e457048947',
        'd8cb9b6c3637ad9be9aff68606ea21d0b484d2d8025fd1988817316e8b3099bd',
        'e2021674783039660bb2a528e57f0a893b1c2c8cd554adf338b8b22880d84cc7', '2025-04-03T03:17:35.336+00:00');
INSERT INTO blocks
VALUES (5, '61e746c65e067c9a05637eda66b29cdc926032560fc360b17414e6fb16b2f04f',
        '3e6bbd66992cd9bbe43e847a8ad3192205676658a20f7840a96ca1e457048947',
        '24768f3b75b8d520a710d585a5b943ff7010e997c60e102ff1baa3f34e6445d1', '2025-04-03T03:17:37.380+00:00');
INSERT INTO blocks
VALUES (6, '69b70e5958f94aee6a571ad36832c56319b2b2817b5f9b2a16b1df01bfbdaa69',
        '61e746c65e067c9a05637eda66b29cdc926032560fc360b17414e6fb16b2f04f',
        '41bedfedac1ef768643a85a3d776d128d9dbb84d01c87071f6a8e9a73debc411', '2025-04-03T03:17:39.389+00:00');
INSERT INTO blocks
VALUES (7, 'bee60c5a76673bb85427852f2730c050667bce1a94b41144365081b3bc2e9b2f',
        '69b70e5958f94aee6a571ad36832c56319b2b2817b5f9b2a16b1df01bfbdaa69',
        'f3f1f46cdf3a15e426e51651149b69b1074e8c110f7ad9576c47c037aed07cfd', '2025-04-03T03:17:41.395+00:00');
INSERT INTO blocks
VALUES (8, '87e9ff2a6f9dac4d680c5b0ffb1b0dad0a9a73f099508a9c84825f0895428d11',
        'bee60c5a76673bb85427852f2730c050667bce1a94b41144365081b3bc2e9b2f',
        '76863ecaed0e72336396c51efa773f9201f1170a3f04a322442b4b541f12b93b', '2025-04-03T03:17:43.405+00:00');
INSERT INTO blocks
VALUES (9, '9e5d8872bff1767ddfc0f893bd238ce0818da39921968ec856fcbb7c6c9b33a9',
        '87e9ff2a6f9dac4d680c5b0ffb1b0dad0a9a73f099508a9c84825f0895428d11',
        'cb6ec94a113b3902c6f239c347a8ac3bd729faf8f32fe21cda31c547fbf868dd', '2025-04-03T03:17:45.420+00:00');
INSERT INTO blocks
VALUES (10, '2d17065e3e145c45627563b0eb6435120fafcb3698d03f3ab461e13c01cc30f7',
        '9e5d8872bff1767ddfc0f893bd238ce0818da39921968ec856fcbb7c6c9b33a9',
        '4c2d6f2d18add456eafde1a0e2cdc2550b10fd129cbde67f46790e4ef43ed71b', '2025-04-03T03:17:47.430+00:00');
INSERT INTO blocks
VALUES (11, '9ff3106e37596b779bfc65257a6a7096619e64baa4640ab7d7226566b74b27dd',
        '2d17065e3e145c45627563b0eb6435120fafcb3698d03f3ab461e13c01cc30f7',
        '661022e55f1b400b0bd7ff132dc16b435d0e53ae0e82322e5143dcf9c788f381', '2025-04-03T03:17:49.445+00:00');
INSERT INTO blocks
VALUES (12, '028c741d93a5def6f700bc718e3fa0ee3cda0843eb254472dfc57770818502a1',
        '9ff3106e37596b779bfc65257a6a7096619e64baa4640ab7d7226566b74b27dd',
        '3f8e419c90203822ac02c5c953233e99914f6e2369a583c428a03abbc8797cb5', '2025-04-03T03:17:51.458+00:00');
INSERT INTO blocks
VALUES (13, 'e491e24ec909e247da9a62adb65d800a2a88a43e9f1cc9854bc9b01db0ccf277',
        '028c741d93a5def6f700bc718e3fa0ee3cda0843eb254472dfc57770818502a1',
        '0be49efe084a32cd0c5026aa915adc8f2cdbc8f222bab7ad94f23f5baf076e63', '2025-04-03T03:17:53.469+00:00');
INSERT INTO blocks
VALUES (14, '6b23d21e80edf64543aa563964d582453f2737022c3bbc32bc8ae70d2de37ba5',
        'e491e24ec909e247da9a62adb65d800a2a88a43e9f1cc9854bc9b01db0ccf277',
        'fa73c4a5b03b46a4665af403ff644324453ecfdea01fd32eadc9a04853c7e485', '2025-04-03T03:17:55.481+00:00');
INSERT INTO blocks
VALUES (15, '8738c4ab9c23309e0bd3e1d421a9f7027d9da9e1e6c31b2368ab764194b54b8b',
        '6b23d21e80edf64543aa563964d582453f2737022c3bbc32bc8ae70d2de37ba5',
        '841d87336f332f7dde52921fd84439d3c0d43e329c75e1fea0f2bbdcff7f55dd', '2025-04-03T03:17:57.489+00:00');
INSERT INTO blocks
VALUES (16, 'd3f9c338771ee3e4e9c2ec3efbd46cd059f178bfeb7b03b042197bc35333c457',
        '8738c4ab9c23309e0bd3e1d421a9f7027d9da9e1e6c31b2368ab764194b54b8b',
        'eb9e19a011a74823bdc9b868abeaa1c3f21c8311d6c10955c1100c4a21afb937', '2025-04-03T03:17:59.498+00:00');
INSERT INTO blocks
VALUES (17, 'd427a74acda7ae3989181812a35ac786538d05c45ac8d29eaccccc3c49307bcb',
        'd3f9c338771ee3e4e9c2ec3efbd46cd059f178bfeb7b03b042197bc35333c457', NULL, '2025-04-03T03:18:01.509+00:00');
CREATE TABLE domains
(
    name     text primary key not null,
    logo     text,
    metadata json
);
INSERT INTO domains
VALUES ('garden_of_live_flowers', NULL, '{}');
INSERT INTO domains
VALUES ('genesis', NULL, '{}');
INSERT INTO domains
VALUES ('looking_glass', '/ipns/QmSrPmbaUKA3ZodhzPWZnpFgcPMFWF4QsxXbkWfEptTBJd', '{
    "important_data": [
        "secret-code",
        1,
        2,
        3
    ],
    "very_important_data": {
        "very": {
            "important": {
                "data": {
                    "is": {
                        "deep": {
                            "inside": 42
                        }
                    }
                }
            }
        }
    }
}');
INSERT INTO domains
VALUES ('wonderland', NULL, '{
    "key": "value"
}');
CREATE TABLE accounts
(
    signatory text not null,
    domain    text not null references domains (name),
    metadata  json,
    primary key (signatory, domain)
);
INSERT INTO accounts
VALUES ('ed0120E9F632D3034BAB6BB26D92AC8FD93EF878D9C5E69E01B61B4C47101884EE2F99', 'garden_of_live_flowers', '{}');
INSERT INTO accounts
VALUES ('ed01204164BF554923ECE1FD412D241036D863A6AE430476C898248B8237D77534CFC4', 'genesis', '{}');
INSERT INTO accounts
VALUES ('ed0120CA92F6B66EB49188C40CE3B4A916687B260518F57C467C2961516F46DE3B537D', 'looking_glass', '{
    "alias": "mad_hatter"
}');
INSERT INTO accounts
VALUES ('ed012004FF5B81046DDCCF19E2E451C45DFB6F53759D4EB30FA2EFA807284D1CC33016', 'wonderland', '{
    "key": "value"
}');
INSERT INTO accounts
VALUES ('ed0120CE7FA46C9DCE7EA4B125E2E36BDB63EA33073E7590AC92816AE1E861B7048B03', 'wonderland', '{
    "alias": "alice",
    "key": "value"
}');
INSERT INTO accounts
VALUES ('ed0120DC3C3FE8DAD83DE7D3C61E21EBC3B25500B0880B894EF75FC385B1DA7203F71C', 'wonderland', '{
    "alias": "bob"
}');
CREATE TABLE domain_owners
(
    account_signatory text not null,
    account_domain    text not null,
    domain            text not null references domains (name),
    foreign key (account_signatory, account_domain) references accounts (signatory, domain),
    primary key (account_signatory, account_domain, domain)
);
INSERT INTO domain_owners
VALUES ('ed01204164BF554923ECE1FD412D241036D863A6AE430476C898248B8237D77534CFC4', 'genesis', 'garden_of_live_flowers');
INSERT INTO domain_owners
VALUES ('ed01204164BF554923ECE1FD412D241036D863A6AE430476C898248B8237D77534CFC4', 'genesis', 'genesis');
INSERT INTO domain_owners
VALUES ('ed0120CE7FA46C9DCE7EA4B125E2E36BDB63EA33073E7590AC92816AE1E861B7048B03', 'wonderland', 'looking_glass');
INSERT INTO domain_owners
VALUES ('ed0120CE7FA46C9DCE7EA4B125E2E36BDB63EA33073E7590AC92816AE1E861B7048B03', 'wonderland', 'wonderland');
CREATE TABLE asset_definitions
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
INSERT INTO asset_definitions
VALUES ('cabbage', 'garden_of_live_flowers', 'ed01204164BF554923ECE1FD412D241036D863A6AE430476C898248B8237D77534CFC4',
        'genesis', NULL, '{}', 'Infinitely');
INSERT INTO asset_definitions
VALUES ('rose', 'wonderland', 'ed0120CE7FA46C9DCE7EA4B125E2E36BDB63EA33073E7590AC92816AE1E861B7048B03', 'wonderland',
        NULL, '{}', 'Infinitely');
CREATE TABLE nfts
(
    name               text not null,
    domain             text not null references domains (name),
    owned_by_signatory text not null,
    owned_by_domain    text not null,
    content            json,
    primary key (name, domain),
    foreign key (owned_by_signatory, owned_by_domain) references accounts (signatory, domain)
);
INSERT INTO nfts
VALUES ('snowflake', 'wonderland', 'ed0120CA92F6B66EB49188C40CE3B4A916687B260518F57C467C2961516F46DE3B537D',
        'looking_glass', '{
        "another-rather-unique-metadata-set-later": [
            5,
            1,
            2,
            3,
            4
        ],
        "what-am-i": "an nft, unique as a snowflake"
    }');
CREATE TABLE assets
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
INSERT INTO assets
VALUES ('cabbage', 'garden_of_live_flowers', 'ed0120CE7FA46C9DCE7EA4B125E2E36BDB63EA33073E7590AC92816AE1E861B7048B03',
        'wonderland', '"44"');
INSERT INTO assets
VALUES ('rose', 'wonderland', 'ed0120CE7FA46C9DCE7EA4B125E2E36BDB63EA33073E7590AC92816AE1E861B7048B03', 'wonderland',
        '"13"');
INSERT INTO assets
VALUES ('rose', 'wonderland', 'ed0120DC3C3FE8DAD83DE7D3C61E21EBC3B25500B0880B894EF75FC385B1DA7203F71C', 'wonderland',
        '"100000"');
CREATE TABLE transactions
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
INSERT INTO transactions
VALUES ('eb9e19a011a74823bdc9b868abeaa1c3f21c8311d6c10955c1100c4a21afb937', 16, '2025-04-03T03:17:57.490+00:00',
        'ed0120CE7FA46C9DCE7EA4B125E2E36BDB63EA33073E7590AC92816AE1E861B7048B03', 'wonderland',
        '8BE09D04975F6B7FDD7AECD2FFE8FE83E23A45BF3E01F4B3B836AB059FDE1F287268BF0FF6DF157EEBE0017FFFAE3F2EE30DE9B8F698BD63FD36DCAED3B93503',
        NULL, '{}', 300000, 'Instructions', '{
        "Validation": {
            "InstructionFailed": {
                "Find": {
                    "Trigger": "ping"
                }
            }
        }
    }');
INSERT INTO transactions
VALUES ('841d87336f332f7dde52921fd84439d3c0d43e329c75e1fea0f2bbdcff7f55dd', 15, '2025-04-03T03:17:55.486+00:00',
        'ed0120CE7FA46C9DCE7EA4B125E2E36BDB63EA33073E7590AC92816AE1E861B7048B03', 'wonderland',
        '2E29730C60E821E1884AC9C9BCAF2D21D37D9858B1B2EE55E51730A4435EC026D9CC1997D611DC49C99122609B13B30C5095897D6FE7D992A50FD572E248E703',
        NULL, '{}', 300000, 'Instructions', '{
        "Validation": {
            "NotPermitted": "unexpected custom instruction"
        }
    }');
INSERT INTO transactions
VALUES ('fa73c4a5b03b46a4665af403ff644324453ecfdea01fd32eadc9a04853c7e485', 14, '2025-04-03T03:17:53.470+00:00',
        'ed0120CE7FA46C9DCE7EA4B125E2E36BDB63EA33073E7590AC92816AE1E861B7048B03', 'wonderland',
        'DACA37C06AD01019FE302FBD27818D01F400E3C9D199128070E951F191513344CC2A2157651917D18B8DAF344C78677C7C4A8657CFDD4FA200298D6872B63C0C',
        NULL, '{}', 300000, 'Instructions', NULL);
INSERT INTO transactions
VALUES ('0be49efe084a32cd0c5026aa915adc8f2cdbc8f222bab7ad94f23f5baf076e63', 13, '2025-04-03T03:17:51.467+00:00',
        'ed0120CE7FA46C9DCE7EA4B125E2E36BDB63EA33073E7590AC92816AE1E861B7048B03', 'wonderland',
        '5840D421793AABEE3318AA0D6DFD284A4A85D6A6337788C38F18827800E82CD66BEF5D17C5CE77F01EB0FF84954404FF657A1833DA791091AA012C15C4360704',
        NULL, '{}', 300000, 'Instructions', '{
        "Validation": {
            "NotPermitted": "Can''t grant or revoke role to another account"
        }
    }');
INSERT INTO transactions
VALUES ('3f8e419c90203822ac02c5c953233e99914f6e2369a583c428a03abbc8797cb5', 12, '2025-04-03T03:17:49.455+00:00',
        'ed0120CE7FA46C9DCE7EA4B125E2E36BDB63EA33073E7590AC92816AE1E861B7048B03', 'wonderland',
        '8CE733ECC1AD1F7B5114EBF223799221E05BBA4E59DB6DBA5C41134E74C9EA436E151CB9919434D510F49B9FE6EE03E36D50227D5488E70140EB77C74D1A620F',
        NULL, '{}', 300000, 'Instructions', NULL);
INSERT INTO transactions
VALUES ('661022e55f1b400b0bd7ff132dc16b435d0e53ae0e82322e5143dcf9c788f381', 11, '2025-04-03T03:17:47.441+00:00',
        'ed0120CE7FA46C9DCE7EA4B125E2E36BDB63EA33073E7590AC92816AE1E861B7048B03', 'wonderland',
        'D686D359D93B7150E0051843EE31B01E5885DB89A24B3E528621C883669172DB610C605748FD69A17C87F5C582AFD7BD925FAA7468D6923BA3D906461D3A380F',
        NULL, '{}', 300000, 'Instructions', NULL);
INSERT INTO transactions
VALUES ('4c2d6f2d18add456eafde1a0e2cdc2550b10fd129cbde67f46790e4ef43ed71b', 10, '2025-04-03T03:17:45.425+00:00',
        'ed0120CE7FA46C9DCE7EA4B125E2E36BDB63EA33073E7590AC92816AE1E861B7048B03', 'wonderland',
        'EF3BF51F395A287660381497D1EB129BEED771BE68FC93FB97DF82F9AEA9A70AE221FB778A1ECDC8DE28CE3B95643ED275213D026B992CB344BE0619BB986E0D',
        NULL, '{}', 300000, 'Instructions', NULL);
INSERT INTO transactions
VALUES ('cb6ec94a113b3902c6f239c347a8ac3bd729faf8f32fe21cda31c547fbf868dd', 9, '2025-04-03T03:17:43.415+00:00',
        'ed0120CE7FA46C9DCE7EA4B125E2E36BDB63EA33073E7590AC92816AE1E861B7048B03', 'wonderland',
        '41475FFB346A56A3DB5D3E4C3AAFA868EB86636F16BAE3B1A73F0203B56D0A2270C5B709B9DFEA859A8E5700B3E387D9E070067566DEF35C5F421C28B7BB2F04',
        NULL, '{}', 300000, 'Instructions', '{
        "Validation": {
            "InstructionFailed": {
                "Find": {
                    "MetadataKey": "non-existing"
                }
            }
        }
    }');
INSERT INTO transactions
VALUES ('76863ecaed0e72336396c51efa773f9201f1170a3f04a322442b4b541f12b93b', 8, '2025-04-03T03:17:41.401+00:00',
        'ed0120CE7FA46C9DCE7EA4B125E2E36BDB63EA33073E7590AC92816AE1E861B7048B03', 'wonderland',
        '173391254B36178E0271EDACF030DFB829B343B9ED8656952D4F06A3D39DC3D3C8C36E1CD782BE8EB9C84D1DB18F9059B3BDA5999ED8061B1C3D2642BB3E390D',
        NULL, '{}', 300000, 'Instructions', NULL);
INSERT INTO transactions
VALUES ('f3f1f46cdf3a15e426e51651149b69b1074e8c110f7ad9576c47c037aed07cfd', 7, '2025-04-03T03:17:39.395+00:00',
        'ed0120CE7FA46C9DCE7EA4B125E2E36BDB63EA33073E7590AC92816AE1E861B7048B03', 'wonderland',
        '30B3E288ED90E020142A6D87C9E29123DA1B453CF56F65A8DE298853E78B148C81D4B2CB59531555313EDE1F71D041A611A04FCE23314397407F6D6C91EC3D0C',
        NULL, '{}', 300000, 'Instructions', NULL);
INSERT INTO transactions
VALUES ('41bedfedac1ef768643a85a3d776d128d9dbb84d01c87071f6a8e9a73debc411', 6, '2025-04-03T03:17:37.383+00:00',
        'ed0120CE7FA46C9DCE7EA4B125E2E36BDB63EA33073E7590AC92816AE1E861B7048B03', 'wonderland',
        '4440242ED6F5A7DF64456CE7D480AACD0AFDF6383FA2F892E2C5446320E0CF466123DFAB365C25A7E32622B7872BF8A015091B628DDC070BF5834101C321B20B',
        NULL, '{}', 300000, 'Instructions', NULL);
INSERT INTO transactions
VALUES ('24768f3b75b8d520a710d585a5b943ff7010e997c60e102ff1baa3f34e6445d1', 5, '2025-04-03T03:17:35.382+00:00',
        'ed0120CE7FA46C9DCE7EA4B125E2E36BDB63EA33073E7590AC92816AE1E861B7048B03', 'wonderland',
        'D61DE4AFC72CD8803CE8DC96079DDEBEECBB5B5AED2E9DFE8500DCDF86CF5F8DD463B67E1AEB3C95384A5D0DE40FCDD6F65B319358B5283B03BFE13D0E972C04',
        NULL, '{}', 300000, 'Instructions', NULL);
INSERT INTO transactions
VALUES ('e2021674783039660bb2a528e57f0a893b1c2c8cd554adf338b8b22880d84cc7', 4, '2025-04-03T03:17:29.356+00:00',
        'ed0120CE7FA46C9DCE7EA4B125E2E36BDB63EA33073E7590AC92816AE1E861B7048B03', 'wonderland',
        '5D2CA6A4C014E28F1A7F074696D6B64183BD51BB7DE284AFBBC9CDAC86F6F78EA54393F3443D244E7C19CCA12D17ADC90A1E3FF7E440476966C5FABCB7345209',
        NULL, '{}', 300000, 'Instructions', NULL);
INSERT INTO transactions
VALUES ('19735c532d67d95971bef55f8a050c39891c9bf13deaf264b4ac072bd0ee3b65', 3, '2025-04-03T03:17:29.306+00:00',
        'ed0120CE7FA46C9DCE7EA4B125E2E36BDB63EA33073E7590AC92816AE1E861B7048B03', 'wonderland',
        'E4FA5CEE122D9AEFA5FC03A78050AC0F0C5EF225229B75B17908C24E93BDE552042708C8BD039CC4E2BC0455159726DBE47122E35FB3BE1BAEEEAABCD2D3D003',
        NULL, '{}', 300000, 'Instructions', NULL);
INSERT INTO transactions
VALUES ('9a176c548ac0cf17f7de500631da46f56d747b474530e80b09edc8dcc0109d5f', 1, '2025-04-03T03:17:13.155+00:00',
        'ed01204164BF554923ECE1FD412D241036D863A6AE430476C898248B8237D77534CFC4', 'genesis',
        'A355240707C781B6057DF4F41FADD57AD3108683DF0D7D41889300053C8D74F41ADF99D7E2A71F47023EBDC62BFFB6E46FE83BBB24EE0F7FB6A8493C26ED5009',
        NULL, '{}', NULL, 'Instructions', NULL);
INSERT INTO transactions
VALUES ('d38ab779d82d58ae5b511060639f63dd6a44a18c6523d48eb75b2d646c45d085', 1, '2025-04-03T03:17:13.159+00:00',
        'ed01204164BF554923ECE1FD412D241036D863A6AE430476C898248B8237D77534CFC4', 'genesis',
        'AC4B656E396929B0B6D269C0466EB3541DF128E6BB4AFE7D1E582F7B20A91BBD2D056CB3180C7264D421FB1BD6DBC4B5FD9868CA4E4F291F7E40B14A784A8905',
        NULL, '{}', NULL, 'Instructions', NULL);
INSERT INTO transactions
VALUES ('981263a44e9600d3b94f315339caf96b49701cd161bf606df85631102aaacb7f', 1, '2025-04-03T03:17:13.159+00:00',
        'ed01204164BF554923ECE1FD412D241036D863A6AE430476C898248B8237D77534CFC4', 'genesis',
        'B110608582616F411274F2B96F2355239637CAB49BCD1DAF7AE8C975BD1A3822BBB768A9D680808FFF3FD43DEDFCA0C9E9C196D31C7C36B3470BF7074B248800',
        NULL, '{}', NULL, 'Instructions', NULL);
INSERT INTO transactions
VALUES ('76d81d851925d8841ec85e20a69ad96e823b951709762b5deb565999cc5f76f7', 1, '2025-04-03T03:17:13.159+00:00',
        'ed01204164BF554923ECE1FD412D241036D863A6AE430476C898248B8237D77534CFC4', 'genesis',
        '6AE269E7E017056008D1B4205C3CBB0B2AD909FB9116B7809010F8D125C3D6C0452113E30B7EE1692EBBA9F8A025941201AB2576EF3E1B07759BCD89560E7B01',
        NULL, '{}', NULL, 'Instructions', NULL);
CREATE TABLE instructions
(
    transaction_hash text not null references transactions (hash),
    value            json not null
);
INSERT INTO instructions
VALUES ('eb9e19a011a74823bdc9b868abeaa1c3f21c8311d6c10955c1100c4a21afb937', '{
    "ExecuteTrigger": {
        "trigger": "ping",
        "args": [
            "do this",
            "then this",
            "and that afterwards"
        ]
    }
}');
INSERT INTO instructions
VALUES ('841d87336f332f7dde52921fd84439d3c0d43e329c75e1fea0f2bbdcff7f55dd', '{
    "Custom": {
        "payload": {
            "kind": "custom",
            "value": false
        }
    }
}');
INSERT INTO instructions
VALUES ('fa73c4a5b03b46a4665af403ff644324453ecfdea01fd32eadc9a04853c7e485', '{
    "Log": {
        "level": "ERROR",
        "msg": "A disrupting message of sorts"
    }
}');
INSERT INTO instructions
VALUES ('0be49efe084a32cd0c5026aa915adc8f2cdbc8f222bab7ad94f23f5baf076e63', '{
    "Revoke": {
        "Role": {
            "object": "RoleThatDoesNotExist",
            "destination": "ed0120DC3C3FE8DAD83DE7D3C61E21EBC3B25500B0880B894EF75FC385B1DA7203F71C@wonderland"
        }
    }
}');
INSERT INTO instructions
VALUES ('3f8e419c90203822ac02c5c953233e99914f6e2369a583c428a03abbc8797cb5', '{
    "Transfer": {
        "Nft": {
            "source": "ed0120CE7FA46C9DCE7EA4B125E2E36BDB63EA33073E7590AC92816AE1E861B7048B03@wonderland",
            "object": "snowflake$wonderland",
            "destination": "ed0120CA92F6B66EB49188C40CE3B4A916687B260518F57C467C2961516F46DE3B537D@looking_glass"
        }
    }
}');
INSERT INTO instructions
VALUES ('661022e55f1b400b0bd7ff132dc16b435d0e53ae0e82322e5143dcf9c788f381', '{
    "Burn": {
        "Asset": {
            "object": "123",
            "destination": "rose##ed0120DC3C3FE8DAD83DE7D3C61E21EBC3B25500B0880B894EF75FC385B1DA7203F71C@wonderland"
        }
    }
}');
INSERT INTO instructions
VALUES ('4c2d6f2d18add456eafde1a0e2cdc2550b10fd129cbde67f46790e4ef43ed71b', '{
    "Mint": {
        "Asset": {
            "object": "100123",
            "destination": "rose##ed0120DC3C3FE8DAD83DE7D3C61E21EBC3B25500B0880B894EF75FC385B1DA7203F71C@wonderland"
        }
    }
}');
INSERT INTO instructions
VALUES ('cb6ec94a113b3902c6f239c347a8ac3bd729faf8f32fe21cda31c547fbf868dd', '{
    "RemoveKeyValue": {
        "Account": {
            "object": "ed0120CA92F6B66EB49188C40CE3B4A916687B260518F57C467C2961516F46DE3B537D@looking_glass",
            "key": "non-existing"
        }
    }
}');
INSERT INTO instructions
VALUES ('76863ecaed0e72336396c51efa773f9201f1170a3f04a322442b4b541f12b93b', '{
    "SetKeyValue": {
        "Nft": {
            "object": "snowflake$wonderland",
            "key": "another-rather-unique-metadata-set-later",
            "value": [
                5,
                1,
                2,
                3,
                4
            ]
        }
    }
}');
INSERT INTO instructions
VALUES ('f3f1f46cdf3a15e426e51651149b69b1074e8c110f7ad9576c47c037aed07cfd', '{
    "SetKeyValue": {
        "Account": {
            "object": "ed0120CE7FA46C9DCE7EA4B125E2E36BDB63EA33073E7590AC92816AE1E861B7048B03@wonderland",
            "key": "alias",
            "value": "alice"
        }
    }
}');
INSERT INTO instructions
VALUES ('41bedfedac1ef768643a85a3d776d128d9dbb84d01c87071f6a8e9a73debc411', '{
    "Register": {
        "Nft": {
            "id": "snowflake$wonderland",
            "content": {
                "what-am-i": "an nft, unique as a snowflake"
            }
        }
    }
}');
INSERT INTO instructions
VALUES ('24768f3b75b8d520a710d585a5b943ff7010e997c60e102ff1baa3f34e6445d1', '{
    "Register": {
        "Account": {
            "id": "ed0120CA92F6B66EB49188C40CE3B4A916687B260518F57C467C2961516F46DE3B537D@looking_glass",
            "metadata": {
                "alias": "mad_hatter"
            }
        }
    }
}');
INSERT INTO instructions
VALUES ('e2021674783039660bb2a528e57f0a893b1c2c8cd554adf338b8b22880d84cc7', '{
    "Register": {
        "Account": {
            "id": "ed0120DC3C3FE8DAD83DE7D3C61E21EBC3B25500B0880B894EF75FC385B1DA7203F71C@wonderland",
            "metadata": {
                "alias": "bob"
            }
        }
    }
}');
INSERT INTO instructions
VALUES ('19735c532d67d95971bef55f8a050c39891c9bf13deaf264b4ac072bd0ee3b65', '{
    "Register": {
        "Domain": {
            "id": "looking_glass",
            "logo": "/ipns/QmSrPmbaUKA3ZodhzPWZnpFgcPMFWF4QsxXbkWfEptTBJd",
            "metadata": {
                "important_data": [
                    "secret-code",
                    1,
                    2,
                    3
                ],
                "very_important_data": {
                    "very": {
                        "important": {
                            "data": {
                                "is": {
                                    "deep": {
                                        "inside": 42
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}');
INSERT INTO instructions
VALUES ('9a176c548ac0cf17f7de500631da46f56d747b474530e80b09edc8dcc0109d5f', '{
    "Upgrade": "MHgwMDk5MjI="
}');
INSERT INTO instructions
VALUES ('d38ab779d82d58ae5b511060639f63dd6a44a18c6523d48eb75b2d646c45d085', '{
    "SetParameter": {
        "Sumeragi": {
            "BlockTimeMs": 2000
        }
    }
}');
INSERT INTO instructions
VALUES ('d38ab779d82d58ae5b511060639f63dd6a44a18c6523d48eb75b2d646c45d085', '{
    "SetParameter": {
        "Sumeragi": {
            "CommitTimeMs": 4000
        }
    }
}');
INSERT INTO instructions
VALUES ('d38ab779d82d58ae5b511060639f63dd6a44a18c6523d48eb75b2d646c45d085', '{
    "SetParameter": {
        "Sumeragi": {
            "MaxClockDriftMs": 1000
        }
    }
}');
INSERT INTO instructions
VALUES ('d38ab779d82d58ae5b511060639f63dd6a44a18c6523d48eb75b2d646c45d085', '{
    "SetParameter": {
        "Block": {
            "MaxTransactions": 512
        }
    }
}');
INSERT INTO instructions
VALUES ('d38ab779d82d58ae5b511060639f63dd6a44a18c6523d48eb75b2d646c45d085', '{
    "SetParameter": {
        "Transaction": {
            "MaxInstructions": 4096
        }
    }
}');
INSERT INTO instructions
VALUES ('d38ab779d82d58ae5b511060639f63dd6a44a18c6523d48eb75b2d646c45d085', '{
    "SetParameter": {
        "Transaction": {
            "SmartContractSize": 4194304
        }
    }
}');
INSERT INTO instructions
VALUES ('d38ab779d82d58ae5b511060639f63dd6a44a18c6523d48eb75b2d646c45d085', '{
    "SetParameter": {
        "Executor": {
            "Fuel": 55000000
        }
    }
}');
INSERT INTO instructions
VALUES ('d38ab779d82d58ae5b511060639f63dd6a44a18c6523d48eb75b2d646c45d085', '{
    "SetParameter": {
        "Executor": {
            "Memory": 55000000
        }
    }
}');
INSERT INTO instructions
VALUES ('d38ab779d82d58ae5b511060639f63dd6a44a18c6523d48eb75b2d646c45d085', '{
    "SetParameter": {
        "SmartContract": {
            "Fuel": 55000000
        }
    }
}');
INSERT INTO instructions
VALUES ('d38ab779d82d58ae5b511060639f63dd6a44a18c6523d48eb75b2d646c45d085', '{
    "SetParameter": {
        "SmartContract": {
            "Memory": 55000000
        }
    }
}');
INSERT INTO instructions
VALUES ('981263a44e9600d3b94f315339caf96b49701cd161bf606df85631102aaacb7f', '{
    "Register": {
        "Domain": {
            "id": "wonderland",
            "logo": null,
            "metadata": {
                "key": "value"
            }
        }
    }
}');
INSERT INTO instructions
VALUES ('981263a44e9600d3b94f315339caf96b49701cd161bf606df85631102aaacb7f', '{
    "Register": {
        "Account": {
            "id": "ed0120CE7FA46C9DCE7EA4B125E2E36BDB63EA33073E7590AC92816AE1E861B7048B03@wonderland",
            "metadata": {
                "key": "value"
            }
        }
    }
}');
INSERT INTO instructions
VALUES ('981263a44e9600d3b94f315339caf96b49701cd161bf606df85631102aaacb7f', '{
    "Register": {
        "Account": {
            "id": "ed012004FF5B81046DDCCF19E2E451C45DFB6F53759D4EB30FA2EFA807284D1CC33016@wonderland",
            "metadata": {
                "key": "value"
            }
        }
    }
}');
INSERT INTO instructions
VALUES ('981263a44e9600d3b94f315339caf96b49701cd161bf606df85631102aaacb7f', '{
    "Register": {
        "AssetDefinition": {
            "id": "rose#wonderland",
            "spec": {
                "scale": null
            },
            "mintable": "Infinitely",
            "logo": null,
            "metadata": {}
        }
    }
}');
INSERT INTO instructions
VALUES ('981263a44e9600d3b94f315339caf96b49701cd161bf606df85631102aaacb7f', '{
    "Register": {
        "Domain": {
            "id": "garden_of_live_flowers",
            "logo": null,
            "metadata": {}
        }
    }
}');
INSERT INTO instructions
VALUES ('981263a44e9600d3b94f315339caf96b49701cd161bf606df85631102aaacb7f', '{
    "Register": {
        "Account": {
            "id": "ed0120E9F632D3034BAB6BB26D92AC8FD93EF878D9C5E69E01B61B4C47101884EE2F99@garden_of_live_flowers",
            "metadata": {}
        }
    }
}');
INSERT INTO instructions
VALUES ('981263a44e9600d3b94f315339caf96b49701cd161bf606df85631102aaacb7f', '{
    "Register": {
        "AssetDefinition": {
            "id": "cabbage#garden_of_live_flowers",
            "spec": {
                "scale": null
            },
            "mintable": "Infinitely",
            "logo": null,
            "metadata": {}
        }
    }
}');
INSERT INTO instructions
VALUES ('981263a44e9600d3b94f315339caf96b49701cd161bf606df85631102aaacb7f', '{
    "Mint": {
        "Asset": {
            "object": "13",
            "destination": "rose##ed0120CE7FA46C9DCE7EA4B125E2E36BDB63EA33073E7590AC92816AE1E861B7048B03@wonderland"
        }
    }
}');
INSERT INTO instructions
VALUES ('981263a44e9600d3b94f315339caf96b49701cd161bf606df85631102aaacb7f', '{
    "Mint": {
        "Asset": {
            "object": "44",
            "destination": "cabbage#garden_of_live_flowers#ed0120CE7FA46C9DCE7EA4B125E2E36BDB63EA33073E7590AC92816AE1E861B7048B03@wonderland"
        }
    }
}');
INSERT INTO instructions
VALUES ('981263a44e9600d3b94f315339caf96b49701cd161bf606df85631102aaacb7f', '{
    "Transfer": {
        "AssetDefinition": {
            "source": "ed01204164BF554923ECE1FD412D241036D863A6AE430476C898248B8237D77534CFC4@genesis",
            "object": "rose#wonderland",
            "destination": "ed0120CE7FA46C9DCE7EA4B125E2E36BDB63EA33073E7590AC92816AE1E861B7048B03@wonderland"
        }
    }
}');
INSERT INTO instructions
VALUES ('981263a44e9600d3b94f315339caf96b49701cd161bf606df85631102aaacb7f', '{
    "Transfer": {
        "Domain": {
            "source": "ed01204164BF554923ECE1FD412D241036D863A6AE430476C898248B8237D77534CFC4@genesis",
            "object": "wonderland",
            "destination": "ed0120CE7FA46C9DCE7EA4B125E2E36BDB63EA33073E7590AC92816AE1E861B7048B03@wonderland"
        }
    }
}');
INSERT INTO instructions
VALUES ('981263a44e9600d3b94f315339caf96b49701cd161bf606df85631102aaacb7f', '{
    "Grant": {
        "Permission": {
            "object": {
                "name": "CanSetParameters",
                "payload": null
            },
            "destination": "ed0120CE7FA46C9DCE7EA4B125E2E36BDB63EA33073E7590AC92816AE1E861B7048B03@wonderland"
        }
    }
}');
INSERT INTO instructions
VALUES ('981263a44e9600d3b94f315339caf96b49701cd161bf606df85631102aaacb7f', '{
    "Grant": {
        "Permission": {
            "object": {
                "name": "CanRegisterDomain",
                "payload": null
            },
            "destination": "ed0120CE7FA46C9DCE7EA4B125E2E36BDB63EA33073E7590AC92816AE1E861B7048B03@wonderland"
        }
    }
}');
INSERT INTO instructions
VALUES ('76d81d851925d8841ec85e20a69ad96e823b951709762b5deb565999cc5f76f7', '{
    "Register": {
        "Peer": "ed01204EE2FCD53E1730AF142D1E23951198678295047F9314B4006B0CB61850B1DB10"
    }
}');
INSERT INTO instructions
VALUES ('76d81d851925d8841ec85e20a69ad96e823b951709762b5deb565999cc5f76f7', '{
    "Register": {
        "Peer": "ed01209897952D14BDFAEA780087C38FF3EB800CB20B882748FC95A575ADB9CD2CB21D"
    }
}');
INSERT INTO instructions
VALUES ('76d81d851925d8841ec85e20a69ad96e823b951709762b5deb565999cc5f76f7', '{
    "Register": {
        "Peer": "ed0120A98BAFB0663CE08D75EBD506FEC38A84E576A7C9B0897693ED4B04FD9EF2D18D"
    }
}');
INSERT INTO instructions
VALUES ('76d81d851925d8841ec85e20a69ad96e823b951709762b5deb565999cc5f76f7', '{
    "Register": {
        "Peer": "ed0120CACF3A84B8DC8710CE9D6B968EE95EC7EE4C93C85858F026F3B4417F569592CE"
    }
}');
CREATE VIEW v_transactions
as
select *,
       format('%s@%s', authority_signatory, authority_domain)       as authority,
       case when error is null then 'committed' else 'rejected' end as status
from transactions;
CREATE VIEW v_instructions
as
select json_each.key                                                as kind,
       instructions.value                                           as box,
       created_at,
       transaction_hash,
       case when error is null then 'committed' else 'rejected' end as transaction_status,
       format('%s@%s', authority_signatory, authority_domain)       as authority,
       block
from instructions,
     json_each(instructions.value)
         join v_transactions as txs on txs.hash = instructions.transaction_hash;
CREATE VIEW v_assets as
select *,
       case assets.definition_domain = assets.owned_by_domain
           when true then format('%s##%s@%s', assets.definition_name, assets.owned_by_signatory, assets.owned_by_domain)
           else format('%s#%s#%s@%s', assets.definition_name, assets.definition_domain, assets.owned_by_signatory,
                       assets.owned_by_domain) end as id,
       value
from assets;
CREATE VIEW v_nfts as
select *,
       format('%s$%s', nfts.name, nfts.domain)                        as id,
       format('%s@%s', nfts.owned_by_signatory, nfts.owned_by_domain) as owned_by
from nfts;
COMMIT;
