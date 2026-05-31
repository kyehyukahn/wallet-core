/**
 * EVM-specific domain types for @kyehyukahn/wallet-core/evm.
 *
 * All uint256 fields are decimal strings (BigInt-safe). nonce and chainId
 * remain `number` for ergonomics — within JS Number.MAX_SAFE_INTEGER they
 * are safe; the package validates this at the boundary in signTransaction.
 */

export type EvmAddress = `0x${string}`;
export type HexString = `0x${string}`;

export interface UnsignedEvmTx {
  /** 20-byte EVM address, 0x-prefixed. */
  to: EvmAddress;
  /** Amount in wei, decimal string. Empty / 0 allowed. */
  value: string;
  /** Call data, 0x-prefixed hex. Omit (or `"0x"`) for plain transfer. */
  data?: HexString;
  /** Transaction nonce. Must be a safe integer. */
  nonce: number;
  /** Gas limit, decimal string. Must be > 0. */
  gasLimit: string;
  /** EIP-155 chain ID. Must be a safe positive integer. */
  chainId: number;
  /** Legacy gas price (wei, decimal). Mutually exclusive with EIP-1559 fields. */
  gasPrice?: string;
  /** EIP-1559 max total fee per gas (wei, decimal). Requires maxPriorityFeePerGas. */
  maxFeePerGas?: string;
  /** EIP-1559 max priority fee (tip) per gas (wei, decimal). Requires maxFeePerGas. */
  maxPriorityFeePerGas?: string;
}
