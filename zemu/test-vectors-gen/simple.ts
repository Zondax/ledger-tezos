import { TezosToolkit } from '@taquito/taquito'
import { LedgerSigner, DerivationType } from '@taquito/ledger-signer'
import TezosApp, { Curve } from '@zondax/ledger-tezos'
import Zemu from '@zondax/zemu'

import { APP_DERIVATION, models, defaultOptions } from '../tests/common'

async function getAddress(app: TezosApp, curve: Curve): Promise<string> {
  const response = await app.getAddressAndPubKey(APP_DERIVATION, curve)

  return response.address
}

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
    Tezos.setProvider({ signer: new LedgerSigner(sim.getTransport(), APP_DERIVATION.slice(2), true, DerivationType.ED25519) })

    console.log(`!!!!!!!!!!!!!!!! Taquito Operation`)
    try {
        const operation = await Tezos.contract.transfer({ to: addresses.k1, amount: 0.01 })
        console.log(`Operation blob: ${operation.raw.opbytes}, object: ${JSON.stringify(operation.raw.opOb)}`)
        console.log(`Operation hash: ${operation.hash}`)
    } catch (e) {
        console.log(`Error issuing operation: ${JSON.stringify(e)}`);
    }
  } finally {
    await sim.close()
  }
}
