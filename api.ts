type Tagged<Tag extends string, Content> = {
  t: Tag;
  c: Content;
};

interface Paginated<T> {
  pagination: {
    page_number: number;
    page_size: number;
    pages: number;
  };
  items: T[];
}

interface Asset {
  account_id: string;
  definition_id: string;
  value: AssetValue;
}

type AssetValue =
  | Tagged<"Quantity", number>
  | Tagged<"BigQuantity", bigint> // be careful! should be deserialized with `json-bigint`
  | Tagged<"Fixed", string> // it's a number too, "float" number, but it cannot fit into js `number`
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
  /**
   * amount of triggers, always 0 for now
   */
  triggers: number;
}

interface PublicKey {
  digest_function: string;
  payload: string;
}

/**
 * This JSON should be parsed with bigint support
 * e.g. https://www.npmjs.com/package/json-bigint
 */
interface Status {
  peers: bigint;
  blocks: bigint;
  txs: bigint;
  uptime: {
    secs: bigint;
    nanos: number;
  };
}
