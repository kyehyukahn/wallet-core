import { requireNativeModule } from 'expo-modules-core';
import { CoinType } from '../src/CoinType';
import type { UnsignedEvmTx, EvmAddress, HexString } from './types';

export type { UnsignedEvmTx, EvmAddress, HexString } from './types';

/** Standard BIP-44 EVM account 0 derivation path. */
export const EVM_DEFAULT_PATH = "m/44'/60'/0'/0/0";

// The native module receives uint256 values as 0x-prefixed hex strings to
// avoid BigInt arithmetic on the Swift/Kotlin side. The TS layer is the
// single place that performs decimal -> hex conversion.
interface EvmTxNative {
  to: string;
  value: HexString;
  data?: HexString;
  nonce: number;
  gasLimit: HexString;
  chainId: number;
  gasPrice?: HexString;
  maxFeePerGas?: HexString;
  maxPriorityFeePerGas?: HexString;
}

interface EvmBindings {
  evmSignTransaction(tx: EvmTxNative, mnemonic: string, path: string): Promise<HexString>;
  deriveAddress(mnemonic: string, coin: number, path: string): Promise<string>;
}

const Native = requireNativeModule('WalletCoreModule') as unknown as EvmBindings;

/**
 * Sign an unsigned EVM transaction with the key derived from `mnemonic`
 * at `derivationPath`. Returns the 0x-prefixed raw signed tx hex,
 * ready for `eth_sendRawTransaction`.
 *
 * Mode (Legacy vs EIP-1559) is inferred from which gas fields are present:
 *   - `gasPrice` → Legacy
 *   - `maxFeePerGas` + `maxPriorityFeePerGas` → EIP-1559 (Enveloped)
 *   - both / neither → throws
 *
 * EIP-2930 (type 1, access-list) is not supported. The caller's adapter
 * should reject `type === 1` transactions upstream.
 */
export function signTransaction(
  tx: UnsignedEvmTx,
  mnemonic: string,
  derivationPath: string = EVM_DEFAULT_PATH,
): Promise<HexString> {
  const native: EvmTxNative = {
    to: tx.to,
    value: decimalToHex(tx.value, 'value'),
    nonce: tx.nonce,
    gasLimit: decimalToHex(tx.gasLimit, 'gasLimit'),
    chainId: tx.chainId,
  };
  if (tx.data !== undefined) native.data = tx.data;
  if (tx.gasPrice !== undefined) native.gasPrice = decimalToHex(tx.gasPrice, 'gasPrice');
  if (tx.maxFeePerGas !== undefined) native.maxFeePerGas = decimalToHex(tx.maxFeePerGas, 'maxFeePerGas');
  if (tx.maxPriorityFeePerGas !== undefined) native.maxPriorityFeePerGas = decimalToHex(tx.maxPriorityFeePerGas, 'maxPriorityFeePerGas');
  return Native.evmSignTransaction(native, mnemonic, derivationPath);
}

/** Derive the EVM address at the given derivation path. */
export function deriveAddress(
  mnemonic: string,
  derivationPath: string = EVM_DEFAULT_PATH,
): Promise<EvmAddress> {
  return Native.deriveAddress(mnemonic, CoinType.Ethereum, derivationPath) as Promise<EvmAddress>;
}

/**
 * Accept decimal-string or 0x-hex input; return canonical 0x-hex. The native
 * side strips leading zeros to produce minimal RLP big-endian bytes.
 */
function decimalToHex(input: string, field: string): HexString {
  if (input.startsWith('0x') || input.startsWith('0X')) {
    return input as HexString;
  }
  let n: bigint;
  try {
    n = BigInt(input);
  } catch {
    throw new Error(`evm.${field}: not a valid integer (${input})`);
  }
  if (n < 0n) {
    throw new Error(`evm.${field}: negative not allowed (${input})`);
  }
  return `0x${n.toString(16)}` as HexString;
}
