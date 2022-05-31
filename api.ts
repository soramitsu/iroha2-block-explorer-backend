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
  mintable: Mintable;
}

type Mintable = "Once" | "Infinitely" | "Not";

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

interface BlockShallow {
  /**
   * Block height, u64
   */
  height: number;
  /**
   * ISO DateTime
   */
  timestamp: string;
  /**
   * WIP zeroed
   */
  block_hash: string;
  /**
   * Transactions count
   */
  transactions: number;
  /**
   * Rejected transactions count
   */
  rejected_transactions: number;
}

interface Block {
  /**
   *
   */
  height: number;
  /**
   * See {@link BlockShallow.timestamp}
   */
  timestamp: string;
  /**
   * WIP as {@link BlockShallow.block_hash}
   */
  block_hash: string;
  parent_block_hash: string;
  rejected_transactions_merkle_root_hash: string;
  invalidated_blocks_hashes: string[];
  /**
   * List of serialized {@link @iroha2/data-model#VersionedValidTransaction}
   */
  transactions: string[];
  /**
   * List of serialized {@link @iroha2/data-model#VersionedRejectedTransaction}
   */
  rejected_transactions: string[];
  /**
   * List of hashes. WIP always empty
   */
  view_change_proofs: string[];
}

type Transaction =
  | Tagged<"Committed", CommittedTransaction>
  | Tagged<"Rejected", RejectedTransaction>;

interface CommittedTransaction {
  /**
   * WIP zeroed
   */
  block_hash: string;
  payload: TransactionPayload;
  signatures: Signature[];
}

interface RejectedTransaction extends CommittedTransaction {
  /**
   * List of serialized {@link @iroha2/data-model#TransactionRejectionReason}
   */
  rejection_reason: string;
}

interface TransactionPayload {
  account_id: string;
  instructions: TransactionInstructions;
  /**
   * ISO timestamp
   */
  creation_time: string;
  /**
   * u64
   */
  time_to_live_ms: number;
  nonce: null | number;
  metadata: any;
}

/**
 * `Instructions` `string[]` - list of serialized
 * {@link @iroha2/data-model#Instruction}
 */
type TransactionInstructions =
  | Tagged<"Instructions", string[]>
  | Tagged<"Wasm", undefined>;

interface Signature {
  /**
   * Public key's multihash
   */
  public_key: string;
  /**
   * Hex binary
   */
  payload: string;
}
