import { OpKind, TezosToolkit } from '@taquito/taquito'
import { ForgeOperationsParams } from '@taquito/rpc'
import { LedgerSigner, DerivationType } from '@taquito/ledger-signer'
import { LocalForger } from '@taquito/local-forging'
import TezosApp, { Curve } from '@zondax/ledger-tezos'
import Zemu, { DeviceModel } from '@zondax/zemu'

const Resolve = require('path').resolve
const createHash = require('crypto').createHash

import { APP_DERIVATION, defaultOptions } from '../tests/common'

import { ledger_fmt, ledger_fmt_currency, MUTEZ_MULT, RPC_ADDR } from './common'

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

    const counterNum = parseInt(counter || '0', 10) + 1 + n

    let delegation_str = addresses.ed
    let delegate: string | undefined = addresses.ed
    if (n % 3 == 0) {
      delegation_str = addresses.p256
      delegate = addresses.p256
    } else if (n % 2 == 0) {
      delegation_str = addresses.k1
      delegate = addresses.k1
    } else if (n % 5 == 0) {
      delegation_str = 'no delegate'
      delegate = undefined
    }

    //prepare operation
    const op: ForgeOperationsParams = {
      branch: hash,
      contents: [
        {
          kind: OpKind.ORIGINATION,
          delegate,
          balance: n.toString(),
          fee: '10000',
          gas_limit: '10',
          storage_limit: '10',
          source,
          counter: counterNum.toString(),
          script: {
            code: [
              {
                prim: 'parameter',
                args: [
                  {
                    prim: 'unit',
                  },
                ],
              },
              {
                prim: 'storage',
                args: [
                  {
                    prim: 'unit',
                  },
                ],
              },
              {
                prim: 'code',
                args: [
                  {
                    prim: 'code',
                  },
                ],
              },
            ],
            storage: {
              prim: 'Unit',
            },
          },
        },
      ],
    }

    const forgedCode = Buffer.from('020000000c0500036c0501036c05020302', 'hex')
    const forgedCodeHash = createHash('sha256').update(forgedCode).digest()
    const forgedStorage = Buffer.from('030b', 'hex')
    const forgedStorageHash = createHash('sha256').update(forgedStorage).digest()

    console.log(`Operation ready, forging... ${JSON.stringify(op)}`)
    //forge the prepared operation
    //this will retrieve the operation blob sent to the ledger device
    const forgedOp = await Tezos.rpc.forgeOperations(op)
    console.log(`Operation blob: ${forgedOp}`)

    //generate test vector with operation and blob
    const test_vector: TestVector = {
      name: `Simple Origination #${n}`,
      blob: forgedOp,
      operation: op,
      output: [
        { idx: 0, key: 'Operation', val: ledger_fmt(hash) }, //page 0
        { idx: 1, key: 'Type', val: ledger_fmt('Origination') }, //page 0
        { idx: 2, key: 'Source', val: ledger_fmt(source) },
        { idx: 3, key: 'Balance', val: ledger_fmt_currency(n.toString()) },
        { idx: 4, key: 'Delegate', val: ledger_fmt(delegation_str) },
        { idx: 5, key: 'Fee', val: ledger_fmt_currency('10000') },
        { idx: 6, key: 'Code', val: ledger_fmt(forgedCodeHash.toString('hex')) },
        { idx: 7, key: 'Storage', val: ledger_fmt(forgedStorageHash.toString('hex')) },
        { idx: 8, key: 'Gas Limit', val: ledger_fmt('10') },
        { idx: 9, key: 'Storage Limit', val: ledger_fmt('10') },
        { idx: 10, key: 'Counter', val: ledger_fmt(counterNum.toString()) },
      ],
    }

    return test_vector
  } finally {
    await sim.close()
  }
}
