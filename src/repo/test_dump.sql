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
VALUES (1, 'fb56a947cd955b7eb1393dfcf9eef5e8a846c107a78dee7497e54188fbf0ddb7', NULL,
        '89b18a9a96f86cf81f2adb2023e223c5671d72f5c22373bc37620e1802ff6a67', '2025-04-02T05:39:06.095+00:00');
INSERT INTO blocks
VALUES (2, '8f264537c0aa889633b734e3b9cc32b14351e91e52036c4661b4f8bff328ce41',
        'fb56a947cd955b7eb1393dfcf9eef5e8a846c107a78dee7497e54188fbf0ddb7', NULL, '2025-04-02T05:39:08.980+00:00');
INSERT INTO blocks
VALUES (3, 'a8ca0b5404678ee54d12494ab5cee4f9411c92dbeb7fc66ad233c4d17490bd2d',
        '8f264537c0aa889633b734e3b9cc32b14351e91e52036c4661b4f8bff328ce41',
        '5cbcbdf1862ceba00fc2abddc7730c05f3c64eff34966b1b70b0ab51d1a648df', '2025-04-02T05:40:22.624+00:00');
INSERT INTO blocks
VALUES (4, '8f2200c93d6203d2a3efbc5f07d1ed236e8a98611edaee620b9f6744d70169f1',
        'a8ca0b5404678ee54d12494ab5cee4f9411c92dbeb7fc66ad233c4d17490bd2d', NULL, '2025-04-02T05:40:22.637+00:00');
INSERT INTO blocks
VALUES (5, '1ebeaad17e57c25f862ac3c1f872db005ca9b355919082658e8018c78bbd4f5b',
        '8f2200c93d6203d2a3efbc5f07d1ed236e8a98611edaee620b9f6744d70169f1',
        'ece8f975bfdf57099a2d12ee1ee9c959cfb656724a2af3bfbcfb719ad934ca79', '2025-04-02T05:40:28.118+00:00');
INSERT INTO blocks
VALUES (6, '1565d9ae134b4fe345ccc897dd95bfc9dcb7c63efbe67fffa7afeac6ebd24171',
        '1ebeaad17e57c25f862ac3c1f872db005ca9b355919082658e8018c78bbd4f5b',
        '6f2d067968375ace8ee2917e2bf6f25ad866e3975a74b25e2135443b6ef24c57', '2025-04-02T05:40:30.132+00:00');
INSERT INTO blocks
VALUES (7, 'e4951502cf8ae5d56593b3ccba124f651824217ded951209e9846345e7f8acb9',
        '1565d9ae134b4fe345ccc897dd95bfc9dcb7c63efbe67fffa7afeac6ebd24171',
        '587bf0b9f04a3fc842c74ce812d6028f6ba772ce201600612f04862fcf52f60b', '2025-04-02T05:40:32.142+00:00');
INSERT INTO blocks
VALUES (8, 'b8661a7bad68745a77fa6e3b763f0ffd67b6f02f6d3e3d80eac974fa57f6d44b',
        'e4951502cf8ae5d56593b3ccba124f651824217ded951209e9846345e7f8acb9',
        '25d5388b04cd30ce3ce43d01b58be0df0b276f1198d508063d33d864a386fb67', '2025-04-02T05:40:34.152+00:00');
INSERT INTO blocks
VALUES (9, '0073a2c392a954cc8d7e461f6b34cb96072d635d5451e492c9aac6c819b743b9',
        'b8661a7bad68745a77fa6e3b763f0ffd67b6f02f6d3e3d80eac974fa57f6d44b',
        '4341104e06e3df7147a89936acea5b9ca143e9c4f9df1c7e94af9a6c14eba6af', '2025-04-02T05:40:36.169+00:00');
INSERT INTO blocks
VALUES (10, '475f18c1be3c9fd6557004381873dd10496f9e8e7019b8dd04202d638dee5059',
        '0073a2c392a954cc8d7e461f6b34cb96072d635d5451e492c9aac6c819b743b9',
        '42415941dab501a6da1de00ca83af5b7514594c66b8fda5a62f744df3b839beb', '2025-04-02T05:40:38.181+00:00');
INSERT INTO blocks
VALUES (11, 'feed156004f49a75929c267bea9ec91d47a914fd7844253970f8e6b8816bbfe5',
        '475f18c1be3c9fd6557004381873dd10496f9e8e7019b8dd04202d638dee5059',
        'd4d18cbb8a01022ec317aa0f41eddff130e3305d13de95e10aa7b318db7a6ff5', '2025-04-02T05:40:40.192+00:00');
INSERT INTO blocks
VALUES (12, '0c9d4b480581300a3d326511af029f9ae8e503a5e7ddf4e581bbbe7d838d03cf',
        'feed156004f49a75929c267bea9ec91d47a914fd7844253970f8e6b8816bbfe5',
        '72026dca9a40b414cb2c86c3aa0346ae07123ec76cc5e54cb86e95d1dbf6bd89', '2025-04-02T05:40:42.199+00:00');
INSERT INTO blocks
VALUES (13, '7edf352272f8b2463de2b0f448d4e580423c6be3b22e188873cad5fe290f2d9f',
        '0c9d4b480581300a3d326511af029f9ae8e503a5e7ddf4e581bbbe7d838d03cf',
        'a17adf28fbd2c2d629adf5f1bd1bc457911c03e481386b80e5bd46d7e993009d', '2025-04-02T05:40:44.211+00:00');
INSERT INTO blocks
VALUES (14, 'd2bf0283156a0358078d5adc609cda2c93dfa80215727511cbf22005a3c82ea3',
        '7edf352272f8b2463de2b0f448d4e580423c6be3b22e188873cad5fe290f2d9f',
        '3b58a72e0bd13a97cd8ed56ae8ca8842125904aec1964e8631c0ea7c2fad5bf1', '2025-04-02T05:40:46.224+00:00');
INSERT INTO blocks
VALUES (15, 'ce592e98e04a3453f1163790090e5a2a3eb5648892c713efdc2a3144d619f3b3',
        'd2bf0283156a0358078d5adc609cda2c93dfa80215727511cbf22005a3c82ea3',
        '47adb45fbc5bb4b04eac9fe44513b8f86a37e79836bca22a9787ab02362e5cef', '2025-04-02T05:40:48.235+00:00');
INSERT INTO blocks
VALUES (16, 'e61db1afb6f1bbe2731e626124fca05ef426e68d48c197231d581757a4743d7d',
        'ce592e98e04a3453f1163790090e5a2a3eb5648892c713efdc2a3144d619f3b3',
        'dc601197faa1fc43ef7b76ceea19f7e54b9a65ba6c4c7366a820d8676b0a94b1', '2025-04-02T05:40:50.244+00:00');
INSERT INTO blocks
VALUES (17, 'c466e1da89e09aa8c60a5011f1bee90b7af651ebcd766682102783695bfe9a9b',
        'e61db1afb6f1bbe2731e626124fca05ef426e68d48c197231d581757a4743d7d',
        '087874c0a680939daf1ed84b2a4d04f41a0cd193f6603dd0722adb68ed799445', '2025-04-02T05:40:52.255+00:00');
INSERT INTO blocks
VALUES (18, '99b844fd8b242d4ec944a0fe2f5cc925183753882a367475dd4db4cb37c8ca4d',
        'c466e1da89e09aa8c60a5011f1bee90b7af651ebcd766682102783695bfe9a9b', NULL, '2025-04-02T05:40:54.272+00:00');
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
VALUES ('ed012048791391BC8BB9662F5E513B168F887F7E1CA6982877168B35963EB31C3A0591', 'looking_glass', '{
    "alias": "mad_hatter"
}');
INSERT INTO accounts
VALUES ('ed012004FF5B81046DDCCF19E2E451C45DFB6F53759D4EB30FA2EFA807284D1CC33016', 'wonderland', '{
    "key": "value"
}');
INSERT INTO accounts
VALUES ('ed012086FB90BDD5CF6979D6B37B9B37AAA94AE39D2F728266A8C09F9DFE8A9E14DA08', 'wonderland', '{
    "alias": "bob"
}');
INSERT INTO accounts
VALUES ('ed0120CE7FA46C9DCE7EA4B125E2E36BDB63EA33073E7590AC92816AE1E861B7048B03', 'wonderland', '{
    "alias": "alice",
    "key": "value"
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
VALUES ('snowflake', 'wonderland', 'ed012048791391BC8BB9662F5E513B168F887F7E1CA6982877168B35963EB31C3A0591',
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
VALUES ('rose', 'wonderland', 'ed012086FB90BDD5CF6979D6B37B9B37AAA94AE39D2F728266A8C09F9DFE8A9E14DA08', 'wonderland',
        '"100000"');
INSERT INTO assets
VALUES ('cabbage', 'garden_of_live_flowers', 'ed0120CE7FA46C9DCE7EA4B125E2E36BDB63EA33073E7590AC92816AE1E861B7048B03',
        'wonderland', '"44"');
INSERT INTO assets
VALUES ('rose', 'wonderland', 'ed0120CE7FA46C9DCE7EA4B125E2E36BDB63EA33073E7590AC92816AE1E861B7048B03', 'wonderland',
        '"13"');
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
VALUES ('087874c0a680939daf1ed84b2a4d04f41a0cd193f6603dd0722adb68ed799445', 17, '2025-04-02T05:40:50.250+00:00',
        'ed0120CE7FA46C9DCE7EA4B125E2E36BDB63EA33073E7590AC92816AE1E861B7048B03', 'wonderland',
        'B2341A825BC83F891B56CCF38B363B248C7AD4F49BD692A8697A4B809C5312C2DFBB3DC2A6A3ADC413903DD46FDA87781BBE540B9AB02F008FEB6728A37DB40B',
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
VALUES ('dc601197faa1fc43ef7b76ceea19f7e54b9a65ba6c4c7366a820d8676b0a94b1', 16, '2025-04-02T05:40:48.242+00:00',
        'ed0120CE7FA46C9DCE7EA4B125E2E36BDB63EA33073E7590AC92816AE1E861B7048B03', 'wonderland',
        '2A8EA1A20D36D27B2E3A9C6D6132FB82B5AA974F770B4FD8C481A9DC00EE1CB3C43E3991AABAAD69E79A1704A3CC7C2B9177EB73EB589B932237FA5672CFE20F',
        NULL, '{}', 300000, 'Instructions', '{
        "Validation": {
            "NotPermitted": "unexpected custom instruction"
        }
    }');
INSERT INTO transactions
VALUES ('47adb45fbc5bb4b04eac9fe44513b8f86a37e79836bca22a9787ab02362e5cef', 15, '2025-04-02T05:40:46.225+00:00',
        'ed0120CE7FA46C9DCE7EA4B125E2E36BDB63EA33073E7590AC92816AE1E861B7048B03', 'wonderland',
        '0D8F3D202D3D650CABC3608B3033E4C6853B150D073D3C1FA1F0B8C150AB60C8FC1A1F0480F22CAC9D7DC130D252E52E6773DB817D7F2CD09C7B2B3E9B4CF80F',
        NULL, '{}', 300000, 'Instructions', NULL);
INSERT INTO transactions
VALUES ('3b58a72e0bd13a97cd8ed56ae8ca8842125904aec1964e8631c0ea7c2fad5bf1', 14, '2025-04-02T05:40:44.221+00:00',
        'ed0120CE7FA46C9DCE7EA4B125E2E36BDB63EA33073E7590AC92816AE1E861B7048B03', 'wonderland',
        '4989017C0F821BF9211050C871D7CD9826B819AA2DEC1869D1C31AAA9DD96AC141ACC11AF1E517C0AA4DA31E75120E98BDA3F5EB6940E419A1026BEB0F1FE10A',
        NULL, '{}', 300000, 'Instructions', '{
        "Validation": {
            "NotPermitted": "Can''t grant or revoke role to another account"
        }
    }');
INSERT INTO transactions
VALUES ('a17adf28fbd2c2d629adf5f1bd1bc457911c03e481386b80e5bd46d7e993009d', 13, '2025-04-02T05:40:42.206+00:00',
        'ed0120CE7FA46C9DCE7EA4B125E2E36BDB63EA33073E7590AC92816AE1E861B7048B03', 'wonderland',
        '06F00F1C25AA25445754D099BADCC2063D52B060AEA49C36AA00DEB1D0CB608485CACD58A10F20ECABD362A67A8EFEC31B011CAF8499C10BAA3E4C0EEAA2E704',
        NULL, '{}', 300000, 'Instructions', NULL);
INSERT INTO transactions
VALUES ('72026dca9a40b414cb2c86c3aa0346ae07123ec76cc5e54cb86e95d1dbf6bd89', 12, '2025-04-02T05:40:40.197+00:00',
        'ed0120CE7FA46C9DCE7EA4B125E2E36BDB63EA33073E7590AC92816AE1E861B7048B03', 'wonderland',
        'F4A98E1B65B5E6C6EEF874FB640128C7078AA08BC72BC589E7D10AA7A3A09F306495E36CF3EB324BF9353614485EAE5D2CF32A9F8D6B2C58411C4FDFBE31460C',
        NULL, '{}', 300000, 'Instructions', NULL);
INSERT INTO transactions
VALUES ('d4d18cbb8a01022ec317aa0f41eddff130e3305d13de95e10aa7b318db7a6ff5', 11, '2025-04-02T05:40:38.183+00:00',
        'ed0120CE7FA46C9DCE7EA4B125E2E36BDB63EA33073E7590AC92816AE1E861B7048B03', 'wonderland',
        '7C46C48ADCD1A65B0E8F3566D7FF233BFF56E4639A29429CD429C640C2DBB9374375EFD86A3C268F8A0360B4B7A7AE9B33CDDA2556B06DFEE140989B8EDBCE0A',
        NULL, '{}', 300000, 'Instructions', NULL);
INSERT INTO transactions
VALUES ('42415941dab501a6da1de00ca83af5b7514594c66b8fda5a62f744df3b839beb', 10, '2025-04-02T05:40:36.180+00:00',
        'ed0120CE7FA46C9DCE7EA4B125E2E36BDB63EA33073E7590AC92816AE1E861B7048B03', 'wonderland',
        'BD9549965066A14217F430C81007A5305F77D04F8D3EC945B77711F1844D3FA35D3A38D0E962994E702B630A4340DA48367F8903B9E9A0F3380B9652719F2D0C',
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
VALUES ('4341104e06e3df7147a89936acea5b9ca143e9c4f9df1c7e94af9a6c14eba6af', 9, '2025-04-02T05:40:34.168+00:00',
        'ed0120CE7FA46C9DCE7EA4B125E2E36BDB63EA33073E7590AC92816AE1E861B7048B03', 'wonderland',
        '70D93B1DF2166FB5A7502E40E8726D4B41ABAEE5319062EBC1E2719F670267690A9BFC6C7222B311F4AF061A9464D1CF4B961A5279C0546338FCDF99E1496B02',
        NULL, '{}', 300000, 'Instructions', NULL);
INSERT INTO transactions
VALUES ('25d5388b04cd30ce3ce43d01b58be0df0b276f1198d508063d33d864a386fb67', 8, '2025-04-02T05:40:32.150+00:00',
        'ed0120CE7FA46C9DCE7EA4B125E2E36BDB63EA33073E7590AC92816AE1E861B7048B03', 'wonderland',
        '7F3ACE2BB6241872CC2D0D53E3D5E632833C011E903FE406AA1376C977EF2E1B817B98A97E258992992638CA5F7B796469B0F9FE4CE3AA2D140C68CB18E39C04',
        NULL, '{}', 300000, 'Instructions', NULL);
INSERT INTO transactions
VALUES ('587bf0b9f04a3fc842c74ce812d6028f6ba772ce201600612f04862fcf52f60b', 7, '2025-04-02T05:40:30.142+00:00',
        'ed0120CE7FA46C9DCE7EA4B125E2E36BDB63EA33073E7590AC92816AE1E861B7048B03', 'wonderland',
        '06CDE117C69910BB3B65D0FD7BCA59E2A236876C5282FB30A34A4507404E5F6352A07C431A729225D73E3D15E2B267D25A963323D8D2B308DE872DD310396E08',
        NULL, '{}', 300000, 'Instructions', NULL);
INSERT INTO transactions
VALUES ('6f2d067968375ace8ee2917e2bf6f25ad866e3975a74b25e2135443b6ef24c57', 6, '2025-04-02T05:40:28.129+00:00',
        'ed0120CE7FA46C9DCE7EA4B125E2E36BDB63EA33073E7590AC92816AE1E861B7048B03', 'wonderland',
        '4474C5AF742ADC0C8C8E035A52ACA0FE5E4F40B148B7629992672AC5E0BA181406CE8A02DEE480EE6817AD77C2ECF4C2D3BC8244907B8A94B6B361DA4B04350D',
        NULL, '{}', 300000, 'Instructions', NULL);
INSERT INTO transactions
VALUES ('ece8f975bfdf57099a2d12ee1ee9c959cfb656724a2af3bfbcfb719ad934ca79', 5, '2025-04-02T05:40:22.661+00:00',
        'ed0120CE7FA46C9DCE7EA4B125E2E36BDB63EA33073E7590AC92816AE1E861B7048B03', 'wonderland',
        'FE7DEB5A9633530CA4DEBF2D387F08189EE6AB6B827553FB5C319CF40F078BCEF9440C6AB739B8DB0C6E9893DA1F44610C4B47A5B289FA989DD10272B8DF5803',
        NULL, '{}', 300000, 'Instructions', NULL);
INSERT INTO transactions
VALUES ('5cbcbdf1862ceba00fc2abddc7730c05f3c64eff34966b1b70b0ab51d1a648df', 3, '2025-04-02T05:40:22.611+00:00',
        'ed0120CE7FA46C9DCE7EA4B125E2E36BDB63EA33073E7590AC92816AE1E861B7048B03', 'wonderland',
        '310E01D5AEF0678105AAD73B98E1047BDD639F40A0C4CA29AD499BC2AA0AE5AE08103FCFE981FBDFA0FBA30B6A74073FFF46CF4ACFEECF6F9479367102457C04',
        NULL, '{}', 300000, 'Instructions', NULL);
INSERT INTO transactions
VALUES ('02c65cf52e0a69da7157ec1716edb02c752f49e5b7841c0eaa10e14c321c7183', 1, '2025-04-02T05:39:06.087+00:00',
        'ed01204164BF554923ECE1FD412D241036D863A6AE430476C898248B8237D77534CFC4', 'genesis',
        '0A91C192E8D35F47364CD9C912477B96CD7FFB28BE3DC2F877E00CEB4DC007B9155D67834C133D94159E32897968742117AC8FBAA79E3EFF42DEBD13E14B6F0E',
        NULL, '{}', NULL, 'Instructions', NULL);
INSERT INTO transactions
VALUES ('ca09f92684b3d12967951078fa66ffdcafaf155284ae6615158530d62f74b74b', 1, '2025-04-02T05:39:06.092+00:00',
        'ed01204164BF554923ECE1FD412D241036D863A6AE430476C898248B8237D77534CFC4', 'genesis',
        '63D6DA62553422A130520F8EF4381091B03E62A606DD18C904A2E4B2E8A88999BB06B8CC38D2B328B2B7F0FC14632DFB2470F09FD82EF0781CB986032CD6920E',
        NULL, '{}', NULL, 'Instructions', NULL);
INSERT INTO transactions
VALUES ('afedd4e6e58c84baeb02b77abc248b475a2f247d7bd80ea4d1bcde5af27c9f93', 1, '2025-04-02T05:39:06.092+00:00',
        'ed01204164BF554923ECE1FD412D241036D863A6AE430476C898248B8237D77534CFC4', 'genesis',
        '98B8F3C1309C0D513FF70D4707E83875C21A9087C871A8B845CD93A8FD7E486B3CF0BC29736CF8BDE2BC142A15F122FF4644E0DCF2233896ED30D39A12575D05',
        NULL, '{}', NULL, 'Instructions', NULL);
INSERT INTO transactions
VALUES ('906c85cc85566c43cf97a7b7e927195615b81c08d1095f47eae98740f92c4e0d', 1, '2025-04-02T05:39:06.092+00:00',
        'ed01204164BF554923ECE1FD412D241036D863A6AE430476C898248B8237D77534CFC4', 'genesis',
        '92C15142C281E2518C4DE0BE5F7ED1F410A7DDE64926C603D4404D08A91E29A71E8A474653603776392F2CB25606800CFB925E6CC443288383BC4A65FF5FE006',
        NULL, '{}', NULL, 'Instructions', NULL);
CREATE TABLE instructions
(
    transaction_hash text not null references transactions (hash),
    value            json not null
);
INSERT INTO instructions
VALUES ('087874c0a680939daf1ed84b2a4d04f41a0cd193f6603dd0722adb68ed799445', '{
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
VALUES ('dc601197faa1fc43ef7b76ceea19f7e54b9a65ba6c4c7366a820d8676b0a94b1', '{
    "Custom": {
        "payload": {
            "kind": "custom",
            "value": false
        }
    }
}');
INSERT INTO instructions
VALUES ('47adb45fbc5bb4b04eac9fe44513b8f86a37e79836bca22a9787ab02362e5cef', '{
    "Log": {
        "level": "ERROR",
        "msg": "A disrupting message of sorts"
    }
}');
INSERT INTO instructions
VALUES ('3b58a72e0bd13a97cd8ed56ae8ca8842125904aec1964e8631c0ea7c2fad5bf1', '{
    "Revoke": {
        "Role": {
            "object": "RoleThatDoesNotExist",
            "destination": "ed012086FB90BDD5CF6979D6B37B9B37AAA94AE39D2F728266A8C09F9DFE8A9E14DA08@wonderland"
        }
    }
}');
INSERT INTO instructions
VALUES ('a17adf28fbd2c2d629adf5f1bd1bc457911c03e481386b80e5bd46d7e993009d', '{
    "Transfer": {
        "Nft": {
            "source": "ed0120CE7FA46C9DCE7EA4B125E2E36BDB63EA33073E7590AC92816AE1E861B7048B03@wonderland",
            "object": "snowflake$wonderland",
            "destination": "ed012048791391BC8BB9662F5E513B168F887F7E1CA6982877168B35963EB31C3A0591@looking_glass"
        }
    }
}');
INSERT INTO instructions
VALUES ('72026dca9a40b414cb2c86c3aa0346ae07123ec76cc5e54cb86e95d1dbf6bd89', '{
    "Burn": {
        "Asset": {
            "object": "123",
            "destination": "rose##ed012086FB90BDD5CF6979D6B37B9B37AAA94AE39D2F728266A8C09F9DFE8A9E14DA08@wonderland"
        }
    }
}');
INSERT INTO instructions
VALUES ('d4d18cbb8a01022ec317aa0f41eddff130e3305d13de95e10aa7b318db7a6ff5', '{
    "Mint": {
        "Asset": {
            "object": "100123",
            "destination": "rose##ed012086FB90BDD5CF6979D6B37B9B37AAA94AE39D2F728266A8C09F9DFE8A9E14DA08@wonderland"
        }
    }
}');
INSERT INTO instructions
VALUES ('42415941dab501a6da1de00ca83af5b7514594c66b8fda5a62f744df3b839beb', '{
    "RemoveKeyValue": {
        "Account": {
            "object": "ed012048791391BC8BB9662F5E513B168F887F7E1CA6982877168B35963EB31C3A0591@looking_glass",
            "key": "non-existing"
        }
    }
}');
INSERT INTO instructions
VALUES ('4341104e06e3df7147a89936acea5b9ca143e9c4f9df1c7e94af9a6c14eba6af', '{
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
VALUES ('25d5388b04cd30ce3ce43d01b58be0df0b276f1198d508063d33d864a386fb67', '{
    "SetKeyValue": {
        "Account": {
            "object": "ed0120CE7FA46C9DCE7EA4B125E2E36BDB63EA33073E7590AC92816AE1E861B7048B03@wonderland",
            "key": "alias",
            "value": "alice"
        }
    }
}');
INSERT INTO instructions
VALUES ('587bf0b9f04a3fc842c74ce812d6028f6ba772ce201600612f04862fcf52f60b', '{
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
VALUES ('6f2d067968375ace8ee2917e2bf6f25ad866e3975a74b25e2135443b6ef24c57', '{
    "Register": {
        "Account": {
            "id": "ed012048791391BC8BB9662F5E513B168F887F7E1CA6982877168B35963EB31C3A0591@looking_glass",
            "metadata": {
                "alias": "mad_hatter"
            }
        }
    }
}');
INSERT INTO instructions
VALUES ('ece8f975bfdf57099a2d12ee1ee9c959cfb656724a2af3bfbcfb719ad934ca79', '{
    "Register": {
        "Account": {
            "id": "ed012086FB90BDD5CF6979D6B37B9B37AAA94AE39D2F728266A8C09F9DFE8A9E14DA08@wonderland",
            "metadata": {
                "alias": "bob"
            }
        }
    }
}');
INSERT INTO instructions
VALUES ('5cbcbdf1862ceba00fc2abddc7730c05f3c64eff34966b1b70b0ab51d1a648df', '{
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
VALUES ('02c65cf52e0a69da7157ec1716edb02c752f49e5b7841c0eaa10e14c321c7183', '{
    "Upgrade": "MHgwMDk5MjI="
}');
INSERT INTO instructions
VALUES ('ca09f92684b3d12967951078fa66ffdcafaf155284ae6615158530d62f74b74b', '{
    "SetParameter": {
        "Sumeragi": {
            "BlockTimeMs": 2000
        }
    }
}');
INSERT INTO instructions
VALUES ('ca09f92684b3d12967951078fa66ffdcafaf155284ae6615158530d62f74b74b', '{
    "SetParameter": {
        "Sumeragi": {
            "CommitTimeMs": 4000
        }
    }
}');
INSERT INTO instructions
VALUES ('ca09f92684b3d12967951078fa66ffdcafaf155284ae6615158530d62f74b74b', '{
    "SetParameter": {
        "Sumeragi": {
            "MaxClockDriftMs": 1000
        }
    }
}');
INSERT INTO instructions
VALUES ('ca09f92684b3d12967951078fa66ffdcafaf155284ae6615158530d62f74b74b', '{
    "SetParameter": {
        "Block": {
            "MaxTransactions": 512
        }
    }
}');
INSERT INTO instructions
VALUES ('ca09f92684b3d12967951078fa66ffdcafaf155284ae6615158530d62f74b74b', '{
    "SetParameter": {
        "Transaction": {
            "MaxInstructions": 4096
        }
    }
}');
INSERT INTO instructions
VALUES ('ca09f92684b3d12967951078fa66ffdcafaf155284ae6615158530d62f74b74b', '{
    "SetParameter": {
        "Transaction": {
            "SmartContractSize": 4194304
        }
    }
}');
INSERT INTO instructions
VALUES ('ca09f92684b3d12967951078fa66ffdcafaf155284ae6615158530d62f74b74b', '{
    "SetParameter": {
        "Executor": {
            "Fuel": 55000000
        }
    }
}');
INSERT INTO instructions
VALUES ('ca09f92684b3d12967951078fa66ffdcafaf155284ae6615158530d62f74b74b', '{
    "SetParameter": {
        "Executor": {
            "Memory": 55000000
        }
    }
}');
INSERT INTO instructions
VALUES ('ca09f92684b3d12967951078fa66ffdcafaf155284ae6615158530d62f74b74b', '{
    "SetParameter": {
        "SmartContract": {
            "Fuel": 55000000
        }
    }
}');
INSERT INTO instructions
VALUES ('ca09f92684b3d12967951078fa66ffdcafaf155284ae6615158530d62f74b74b', '{
    "SetParameter": {
        "SmartContract": {
            "Memory": 55000000
        }
    }
}');
INSERT INTO instructions
VALUES ('afedd4e6e58c84baeb02b77abc248b475a2f247d7bd80ea4d1bcde5af27c9f93', '{
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
VALUES ('afedd4e6e58c84baeb02b77abc248b475a2f247d7bd80ea4d1bcde5af27c9f93', '{
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
VALUES ('afedd4e6e58c84baeb02b77abc248b475a2f247d7bd80ea4d1bcde5af27c9f93', '{
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
VALUES ('afedd4e6e58c84baeb02b77abc248b475a2f247d7bd80ea4d1bcde5af27c9f93', '{
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
VALUES ('afedd4e6e58c84baeb02b77abc248b475a2f247d7bd80ea4d1bcde5af27c9f93', '{
    "Register": {
        "Domain": {
            "id": "garden_of_live_flowers",
            "logo": null,
            "metadata": {}
        }
    }
}');
INSERT INTO instructions
VALUES ('afedd4e6e58c84baeb02b77abc248b475a2f247d7bd80ea4d1bcde5af27c9f93', '{
    "Register": {
        "Account": {
            "id": "ed0120E9F632D3034BAB6BB26D92AC8FD93EF878D9C5E69E01B61B4C47101884EE2F99@garden_of_live_flowers",
            "metadata": {}
        }
    }
}');
INSERT INTO instructions
VALUES ('afedd4e6e58c84baeb02b77abc248b475a2f247d7bd80ea4d1bcde5af27c9f93', '{
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
VALUES ('afedd4e6e58c84baeb02b77abc248b475a2f247d7bd80ea4d1bcde5af27c9f93', '{
    "Mint": {
        "Asset": {
            "object": "13",
            "destination": "rose##ed0120CE7FA46C9DCE7EA4B125E2E36BDB63EA33073E7590AC92816AE1E861B7048B03@wonderland"
        }
    }
}');
INSERT INTO instructions
VALUES ('afedd4e6e58c84baeb02b77abc248b475a2f247d7bd80ea4d1bcde5af27c9f93', '{
    "Mint": {
        "Asset": {
            "object": "44",
            "destination": "cabbage#garden_of_live_flowers#ed0120CE7FA46C9DCE7EA4B125E2E36BDB63EA33073E7590AC92816AE1E861B7048B03@wonderland"
        }
    }
}');
INSERT INTO instructions
VALUES ('afedd4e6e58c84baeb02b77abc248b475a2f247d7bd80ea4d1bcde5af27c9f93', '{
    "Transfer": {
        "AssetDefinition": {
            "source": "ed01204164BF554923ECE1FD412D241036D863A6AE430476C898248B8237D77534CFC4@genesis",
            "object": "rose#wonderland",
            "destination": "ed0120CE7FA46C9DCE7EA4B125E2E36BDB63EA33073E7590AC92816AE1E861B7048B03@wonderland"
        }
    }
}');
INSERT INTO instructions
VALUES ('afedd4e6e58c84baeb02b77abc248b475a2f247d7bd80ea4d1bcde5af27c9f93', '{
    "Transfer": {
        "Domain": {
            "source": "ed01204164BF554923ECE1FD412D241036D863A6AE430476C898248B8237D77534CFC4@genesis",
            "object": "wonderland",
            "destination": "ed0120CE7FA46C9DCE7EA4B125E2E36BDB63EA33073E7590AC92816AE1E861B7048B03@wonderland"
        }
    }
}');
INSERT INTO instructions
VALUES ('afedd4e6e58c84baeb02b77abc248b475a2f247d7bd80ea4d1bcde5af27c9f93', '{
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
VALUES ('afedd4e6e58c84baeb02b77abc248b475a2f247d7bd80ea4d1bcde5af27c9f93', '{
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
VALUES ('906c85cc85566c43cf97a7b7e927195615b81c08d1095f47eae98740f92c4e0d', '{
    "Register": {
        "Peer": "ed01204EE2FCD53E1730AF142D1E23951198678295047F9314B4006B0CB61850B1DB10"
    }
}');
INSERT INTO instructions
VALUES ('906c85cc85566c43cf97a7b7e927195615b81c08d1095f47eae98740f92c4e0d', '{
    "Register": {
        "Peer": "ed01209897952D14BDFAEA780087C38FF3EB800CB20B882748FC95A575ADB9CD2CB21D"
    }
}');
INSERT INTO instructions
VALUES ('906c85cc85566c43cf97a7b7e927195615b81c08d1095f47eae98740f92c4e0d', '{
    "Register": {
        "Peer": "ed0120A98BAFB0663CE08D75EBD506FEC38A84E576A7C9B0897693ED4B04FD9EF2D18D"
    }
}');
INSERT INTO instructions
VALUES ('906c85cc85566c43cf97a7b7e927195615b81c08d1095f47eae98740f92c4e0d', '{
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
CREATE VIEW v_assets as
select case assets.definition_domain = assets.owned_by_domain
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
