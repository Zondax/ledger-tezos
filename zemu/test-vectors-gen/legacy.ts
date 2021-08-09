import { OpKind, TezosToolkit } from '@taquito/taquito'
import { ForgeOperationsParams } from '@taquito/rpc'
import { LedgerSigner, DerivationType } from '@taquito/ledger-signer'
import TezosApp, { Curve } from '@zondax/ledger-tezos'
import Zemu, { DeviceModel } from '@zondax/zemu'

const Resolve = require('path').resolve;

import { APP_DERIVATION, defaultOptions } from '../tests/common'

import { ledger_fmt } from './common';

const MUTEZ_MULT = 1_000_000;

async function getAddress(app: TezosApp, curve: Curve): Promise<string> {
  const response = await app.legacyGetPubKey(APP_DERIVATION, curve)

  return response.address
}

const models: DeviceModel[] = [
  { name: 'nanos', prefix: 'S', path: Resolve('../legacy/output/app.elf') },
];

export async function run(n: number): Promise<TestVector[]> {
  const vectors = [];

  for(let i = 0; i < n; i ++) {
    vectors.push(await generate_vector(i))
  }

  return vectors;
}

export type ExpectedPage = { idx: number, key: string, val: string[] };

export type TestVector = {
  name: string,
  blob: string,
  output: Array<ExpectedPage>
}

async function generate_vector(n: number): Promise<TestVector> {
  //start emulator
  const m = models[0]
  const sim = new Zemu(m.path)
  try {
    await sim.start({ ...defaultOptions, model: m.name })

    const app = new TezosApp(sim.getTransport())
    const addresses = {
      ed10: await getAddress(app, Curve.Ed25519_Slip10),
      ed: await getAddress(app, Curve.Ed25519),
      k1: await getAddress(app, Curve.Secp256K1),
      //p256: await getAddress(app, Curve.Secp256R1),
    }
    console.log(`populated addresses: ${JSON.stringify(addresses)}`);

    //check that we have enough balance

    //get taquito toolkit and set ledger signer
    const Tezos = new TezosToolkit('https://granadanet.tezos.dev.zondax.net')

    //slice to skip "m/" which is not wanted by taquito
    //false so prompt is optional
    //derivation type is optional but we specify for clarity
    Tezos.setProvider({ signer: new LedgerSigner(sim.getTransport(), APP_DERIVATION.slice(2), false, DerivationType.ED25519) })

    //1 tezos = 1'000'000 mutez (micro tez)
    // estimate fees of operation (alternatively can be set manually)
    const estimate = await Tezos.estimate.transfer({to: addresses.k1, amount: 0.01, mutez: false})

    const source = await Tezos.signer.publicKeyHash();

    const { counter } = await Tezos.rpc.getContract(source);
    //branch is the block block hash we want to submit this transaction to
    const { hash } = await Tezos.rpc.getBlockHeader();

    const amount = 0.01;

    //prepare operation
    const op: ForgeOperationsParams = {
      branch: hash,
      contents: [{
        kind: OpKind.TRANSACTION,
        destination: addresses.k1,
        amount: (amount * MUTEZ_MULT).toString(), //has to be in mutez
        fee: estimate.suggestedFeeMutez.toString(),
        gas_limit: estimate.gasLimit.toString(),
        storage_limit: estimate.storageLimit.toString(),
        source,
        counter: (parseInt(counter || '0', 10) + 1).toString(),
      }]
    };

    console.log(`Operation ready, forging... ${JSON.stringify(op)}`)
    //forge the prepared operation
    //this will retrieve the operation blob sent to the ledger device
    const forgedOp = await Tezos.rpc.forgeOperations(op)
    console.log(`Operation blob: ${forgedOp}`);

    //generate test vector with operation and blob
    const test_vector: TestVector = {
      name: `Simple TX #${n}`,
      blob: forgedOp,
      output: [
        { idx: 0, key: "Kind", val: ledger_fmt("Transaction") }, //page 0
        { idx: 1, key: "Amount", val: ledger_fmt(amount.toString())},
        { idx: 2, key: "Fee", val: ledger_fmt(estimate.suggestedFeeMutez.toString()) },
        { idx: 3, key: "Source", val: ledger_fmt(source) },
        { idx: 4, key: "Destination", val: ledger_fmt(addresses.k1) },
        { idx: 5, key: "Storage limit", val: ledger_fmt(estimate.storageLimit.toString()) },
      ]
    };

    return test_vector;
  } finally {
    await sim.close()
  }
}

/**
 Example operation (legacy)

 payload(hex):
 CLA INS P1 P2 PLEN
 80 04 81 00 58
 03e11258d2d3a574f86ce556e9a779b80371d2416c20e3868341b918861f0ef5f56c009a6090844356d979899622d85ba1602740fcaa84ba03abc939f70b00904e00018907e2009bc7da38c9cf0cf97bb331ef86d5392b00

 Output:
 Amount: 0.01
 Fee: 0.000442
 Source: tz address (pages)
 Destination: tz address (pages)
 Storage limit: 0
 * * */
