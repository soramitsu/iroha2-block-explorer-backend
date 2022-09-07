export type Tagged<Tag extends string, Content> = {
  t: Tag;
  c: Content;
};

export interface Paginated<T> {
  pagination: {
    page_number: number;
    page_size: number;
    pages: number;
  };
  items: T[];
}

export interface Asset {
  account_id: string;
  definition_id: string;
  value: AssetValue;
}

// There is no need to work with values as with numbers on the frontend. 
// Some values may exceed the allowable number sizes in js
export type AssetValue =
  | Tagged<"Quantity", string>
  | Tagged<"BigQuantity", string>
  | Tagged<"Fixed", string> 
  | Tagged<"Store", any>;

export type AssetValueType = "Quantity" | "BigQuantity" | "Fixed" | "Store";

export interface AssetDefinition {
  id: string;
  value_type: AssetValueType;
  mintable: Mintable;
}

export interface AssetDefinitionWithAccounts extends AssetDefinition {
  /**
   * List of account IDs
   */
  accounts: string[];
}

export type Mintable = "Once" | "Infinitely" | "Not";

export interface Peer {
  address: string;
  public_key: PublicKey;
}

export interface Role {
  id: string;
  permissions: PermissionToken[];
}

export interface PermissionToken {
  name: string;
  params: any;
}

export interface Account {
  id: string;
  assets: Asset[];
  signatories: PublicKey[];
  permission_tokens: PermissionToken[];
  roles: Role[];
  signature_check_condition: any;
  metadata: any;
}

export interface Domain {
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

export interface PublicKey {
  digest_function: string;
  payload: string;
}

// There is no need to work with values as with numbers on the frontend. 
// Some values may exceed the allowable number sizes in js
export interface Status {
  peers: string;
  blocks: string;
  txs: string;
  uptime: {
    // Despite the fact that u64 is here, the value will not exceed the allowable number in js
    secs: number; 
    nanos: number;
  };
}

export interface BlockShallow {
  height: number;
  /**
   * ISO DateTime
   */
  timestamp: string;
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

export interface Block {
  height: number;
  /**
   * See {@link BlockShallow.timestamp}
   */
  timestamp: string;
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

export type Transaction =
  | Tagged<"Committed", CommittedTransaction>
  | Tagged<"Rejected", RejectedTransaction>;

export interface CommittedTransaction {
  /**
   * WIP zeroed
   */
  block_hash: string;
  payload: TransactionPayload;
  signatures: Signature[];
}

export interface RejectedTransaction extends CommittedTransaction {
  /**
   * List of serialized {@link @iroha2/data-model#TransactionRejectionReason}
   */
  rejection_reason: string;
}

export interface TransactionPayload {
  account_id: string;
  instructions: TransactionInstructions;
  /**
   * ISO timestamp
   */
  creation_time: string;
  time_to_live_ms: number;
  nonce: null | number;
  metadata: any;
}

/**
 * `Instructions` `string[]` - list of serialized
 * {@link @iroha2/data-model#Instruction}
 */
export type TransactionInstructions =
  | Tagged<"Instructions", string[]>
  | Tagged<"Wasm", undefined>;

export interface Signature {
  /**
   * Public key's multihash
   */
  public_key: string;
  /**
   * Hex binary
   */
  payload: string;
}
