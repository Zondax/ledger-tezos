import { OpKind, TezosToolkit } from '@taquito/taquito'
import { ForgeOperationsParams } from '@taquito/rpc'
import { LedgerSigner, DerivationType } from '@taquito/ledger-signer'
import { LocalForger } from '@taquito/local-forging'
import TezosApp, { Curve } from '@zondax/ledger-tezos'
import Zemu, { DeviceModel } from '@zondax/zemu'

const Resolve = require('path').resolve

import { APP_DERIVATION, defaultOptions } from '../tests/common'

import { ledger_fmt } from './common'

const MUTEZ_MULT = 1_000_000

async function getAddress(app: TezosApp, curve: Curve): Promise<string> {
  const response = await app.getAddressAndPubKey(APP_DERIVATION, curve)

  return response.address
}

const models: DeviceModel[] = [{ name: 'nanos', prefix: 'S', path: Resolve('../rust/app/output/app_s.elf') }]

export async function run(n: number): Promise<TestVector[]> {
  const vectors = []

  for (let i = 0; i < n; i++) {
    vectors.push(await generate_vector(i))
  }

  return vectors
}

export type ExpectedPage = { idx: number; key: string; val: string[] }

export type TestVector = {
  name: string
  blob: string
  output: Array<ExpectedPage>
  operation: ForgeOperationsParams
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
    console.log(`populated addresses: ${JSON.stringify(addresses)}`)

    //check that we have enough balance

    //get taquito toolkit and set ledger signer
    const Tezos = new TezosToolkit('https://granadanet.tezos.dev.zondax.net')

    let derivation_type = DerivationType.ED25519;
    if (n % 3 == 0) {
      derivation_type = DerivationType.SECP256K1
    } else if (n % 2 == 0) {
      derivation_type = DerivationType.P256
    }

    //slice to skip "m/" which is not wanted by taquito
    //false so prompt is optional
    //derivation type is optional but we specify for clarity
    Tezos.setProvider({
      signer: new LedgerSigner(sim.getTransport(), APP_DERIVATION.slice(2), false, derivation_type),
      forger: new LocalForger(),
    })

    // estimate fees of operation (alternatively can be set manually)
    const mby_estimate = await Tezos.estimate.reveal();
    const estimate = mby_estimate ? mby_estimate : { gasLimit: 10, storageLimit: 100, suggestedFeeMutez: 1234 };

    const source = await Tezos.signer.publicKeyHash().catch(e => { console.log(e); throw e });
    const pk = await Tezos.signer.publicKey().catch(e => {console.log(e); throw e});

    const { counter } = await Tezos.rpc.getContract(source)
    //branch is the block block hash we want to submit this transaction to
    const { hash } = await Tezos.rpc.getBlockHeader()

    const counterNum = (parseInt(counter || '0', 10) + 1 + n);

    //prepare operation
    const op: ForgeOperationsParams = {
      branch: hash,
      contents: [
        {
          kind: OpKind.REVEAL,
          fee: estimate.suggestedFeeMutez.toString(),
          gas_limit: estimate.gasLimit.toString(),
          storage_limit: estimate.storageLimit.toString(),
          source,
          counter: counterNum.toString(),
          public_key: pk,
        },
      ],
    }

    console.log(`Operation ready, forging... ${JSON.stringify(op)}`)
    //forge the prepared operation
    //this will retrieve the operation blob sent to the ledger device
    const forgedOp = await Tezos.rpc.forgeOperations(op)
    console.log(`Operation blob: ${forgedOp}`)

    //generate test vector with operation and blob
    const test_vector: TestVector = {
      name: `Simple Reveal #${n}`,
      blob: forgedOp,
      operation: op,
      output: [
        { idx: 0, key: 'Operation', val: ledger_fmt(hash) }, //page 0
        { idx: 1, key: 'Type', val: ledger_fmt("Revelation") }, //page 0
        { idx: 2, key: 'Source', val: ledger_fmt(source) },
        { idx: 3, key: 'Public Key', val: ledger_fmt(pk) },
        { idx: 4, key: 'Fee', val: ledger_fmt(estimate.suggestedFeeMutez.toString()) },
        { idx: 5, key: 'Gas Limit', val: ledger_fmt(estimate.gasLimit.toString()) },
        { idx: 6, key: 'Storage Limit', val: ledger_fmt(estimate.storageLimit.toString()) },
        { idx: 7, key: 'Counter', val: ledger_fmt(counterNum.toString()) },
      ],
    }

    return test_vector
  } finally {
    await sim.close()
  }
}
