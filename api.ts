type Tagged<Tag extends string, Content> = {
  t: Tag;
  c: Content;
};

interface Asset {
  account_id: string;
  definition_id: string;
  value: AssetValue;
}

type AssetValue =
  | Tagged<"Quantity", number>
  | Tagged<"BigQuantity", string>
  | Tagged<"Fixed", string>
  | Tagged<"Store", any>;

type AssetValueType = "Quantity" | "BigQuantity" | "Fixed" | "Store";

interface AssetDefinition {
  id: string;
  value_type: AssetValueType;
  mintable: boolean;
}

interface Peer {
  address: string;
  public_key: PublicKey;
}

interface Role {
  id: string;
  permissions: PermissionToken[];
}

interface PermissionToken {
  name: string;
  params: any;
}

interface Account {
  id: string;
  assets: Asset[];
  signatories: PublicKey[];
  permission_tokens: PermissionToken[];
  roles: Role[];
  signature_check_condition: any;
  metadata: any;
}

interface Domain {
  id: string;
  accounts: Account[];
  asset_definitions: AssetDefinition[];
  logo: null | string;
  metadata: any;
}

interface PublicKey {
  digest_function: string;
  payload: string;
}

/**
 * `peers`, `blocks` and `txs` are u64 numbers,
 * so JSON parsing may fail if they are larger than MAX_U32
 * (which fits into `f64` JS number)
 */
interface Status {
    peers: number
    blocks: number
    txs: number
    uptime: {
        secs: number
        /**
         * only this is u32
         */
        nanos: number
    }
}
