import { OpKind, TezosToolkit } from '@taquito/taquito'
import { ForgeOperationsParams, OperationContentsBallotEnum } from '@taquito/rpc'
import { LedgerSigner, DerivationType } from '@taquito/ledger-signer'
import { LocalForger } from '@taquito/local-forging'
import TezosApp, { Curve } from '@zondax/ledger-tezos'
import Zemu, { DeviceModel } from '@zondax/zemu'

const Resolve = require('path').resolve

import { APP_DERIVATION, defaultOptions } from '../tests/common'

import { ledger_fmt, RPC_ADDR } from './common'

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
      p256: await getAddress(app, Curve.Secp256R1),
    }
    console.log(`populated addresses: ${JSON.stringify(addresses)}`)

    //check that we have enough balance

    //get taquito toolkit and set ledger signer
    const Tezos = new TezosToolkit(RPC_ADDR)

    //slice to skip "m/" which is not wanted by taquito
    //false so prompt is optional
    //derivation type is optional but we specify for clarity
    Tezos.setProvider({
      signer: new LedgerSigner(sim.getTransport(), APP_DERIVATION.slice(2), false, DerivationType.ED25519),
      forger: new LocalForger(),
    })

    const source = await Tezos.signer.publicKeyHash()

    const { counter } = await Tezos.rpc.getContract(source)
    //branch is the block block hash we want to submit this transaction to
    const { hash } = await Tezos.rpc.getBlockHeader()

    const counterNum = (parseInt(counter || '0', 10) + 1 + n);

    const PROPOSAL_01 = "PsCARTHAGazKbHtnKfLzQg3kms52kSRpgnDY982a9oYsSXRLQEb";
    const PROPOSAL_02 = "PsFLorenaUUuikDWvMDr6fGBRG8kt3e3D3fHoXK1j1BFRxeSH4i";

    let proposals;
    if (n % 2 == 0) {
      proposals = [PROPOSAL_01, PROPOSAL_02]
    } else {
      proposals = [PROPOSAL_02, PROPOSAL_01]
    }

    //prepare operation
    const op: ForgeOperationsParams = {
      branch: hash,
      contents: [
        {
          kind: OpKind.PROPOSALS,
          source,
          period: n,
          proposals,
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
      name: `Simple Proposals #${n}`,
      blob: forgedOp,
      operation: op,
      output: [
        { idx: 0, key: 'Operation', val: ledger_fmt(hash) }, //page 0
        { idx: 1, key: 'Type', val: ledger_fmt("Proposals") }, //page 0
        { idx: 2, key: 'Source', val: ledger_fmt(source) },
        { idx: 3, key: 'Period', val: ledger_fmt(n.toString()) },
        { idx: 4, key: 'Proposal #1', val: ledger_fmt(proposals[0]) },
        { idx: 5, key: 'Proposal #2', val: ledger_fmt(proposals[1]) },
      ],
    }

    return test_vector
  } finally {
    await sim.close()
  }
}
