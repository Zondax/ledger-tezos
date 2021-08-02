import { OpKind, TezosToolkit } from '@taquito/taquito'
import { ForgeOperationsParams } from '@taquito/rpc'
import { LedgerSigner, DerivationType } from '@taquito/ledger-signer'
import TezosApp, { Curve } from '@zondax/ledger-tezos'
import Zemu, { DeviceModel } from '@zondax/zemu'

const Resolve = require('path').resolve;

import { APP_DERIVATION, defaultOptions } from '../tests/common'

const MUTEZ_MULT = 0.000_001;

async function getAddress(app: TezosApp, curve: Curve): Promise<string> {
  const response = await app.legacyGetPubKey(APP_DERIVATION, curve)

  return response.address
}

const models: DeviceModel[] = [
  { name: 'nanos', prefix: 'S', path: Resolve('../legacy/output/app.elf') },
];

export async function run() {
  //start emulator
  const m = models[0]
  const sim = new Zemu(m.path)
  try {
    await sim.start({ ...defaultOptions, model: m.name, X11: true })

    const app = new TezosApp(sim.getTransport())
    const addresses = {
      ed: await getAddress(app, Curve.Ed25519_Slip10),
      k1: await getAddress(app, Curve.Secp256K1),
      //p256: await getAddress(app, Curve.Secp256R1),
    }
    console.log(`populated addresses: ${JSON.stringify(addresses)}`);

    //check that we have enough balance

    //get taquito toolkit and set ledger signer
    const Tezos = new TezosToolkit('https://testnet-tezos.giganode.io')

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

    //prepare operation
    const op: ForgeOperationsParams = {
      branch: hash,
      contents: [{
        kind: OpKind.TRANSACTION,
        destination: addresses.k1,
        amount: (0.01 * MUTEZ_MULT).toString(), //has to be in mutez
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

    console.log(`Operation blob (?): ${forgedOp}`);

    //get the signature
    const sig = await Tezos.signer.sign(forgedOp, new Uint8Array([3]))
    console.log(`Signed operation object: ${JSON.stringify(sig)}`);

  } catch (e) {
    console.log(`Exception occurred: ${JSON.stringify(e)}`)
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
