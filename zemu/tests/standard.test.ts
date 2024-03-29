/** ******************************************************************************
 *  (c) 2020 Zondax GmbH
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 ******************************************************************************* */

import Zemu from '@zondax/zemu'
import { APP_DERIVATION, cartesianProduct, curves, defaultOptions, models } from './common'
import TezosApp, { Curve } from '@zondax/ledger-tezos'
import * as secp256k1 from 'noble-secp256k1'

import { SAMPLE_TRANSACTION, KNOWN_DELEGATE } from './tezos'

const ed25519 = require('ed25519-supercop')

describe.each(models)('Standard', function (m) {
  test('can start and stop container', async function () {
    const sim = new Zemu(m.path)
    try {
      await sim.start({ ...defaultOptions, model: m.name })
    } finally {
      await sim.close()
    }
  })

  test('main menu', async function () {
    const sim = new Zemu(m.path)
    try {
      await sim.start({ ...defaultOptions, model: m.name })
      await sim.navigateAndCompareSnapshots('.', `${m.prefix.toLowerCase()}-mainmenu`, [1, 0, 0, 4, -5])
    } finally {
      await sim.close()
    }
  })

  test('get app version', async function () {
    const sim = new Zemu(m.path)
    try {
      await sim.start({ ...defaultOptions, model: m.name })
      const app = new TezosApp(sim.getTransport())
      const resp = await app.getVersion()

      console.log(resp)

      expect(resp.returnCode).toEqual(0x9000)
      expect(resp.errorMessage).toEqual('No errors')
      expect(resp).toHaveProperty('testMode')
      expect(resp).toHaveProperty('major')
      expect(resp).toHaveProperty('minor')
      expect(resp).toHaveProperty('patch')
    } finally {
      await sim.close()
    }
  })
})

describe.each(models)('Standard [%s]; legacy', function (m) {
  test('get app version', async function () {
    const sim = new Zemu(m.path)
    try {
      await sim.start({ ...defaultOptions, model: m.name })
      const app = new TezosApp(sim.getTransport())
      const resp = await app.legacyGetVersion()

      console.log(resp)

      expect(resp.returnCode).toEqual(0x9000)
      expect(resp.errorMessage).toEqual('No errors')
      expect(resp).toHaveProperty('baking')
      expect(resp.baking).toBe(false)
      expect(resp).toHaveProperty('major')
      expect(resp).toHaveProperty('minor')
      expect(resp).toHaveProperty('patch')
    } finally {
      await sim.close()
    }
  })

  test('get git app', async function () {
    const sim = new Zemu(m.path)
    try {
      await sim.start({ ...defaultOptions, model: m.name })
      const app = new TezosApp(sim.getTransport())
      const resp = await app.legacyGetGit()

      console.log(resp)
      expect(resp.returnCode).toEqual(0x9000)
      expect(resp.errorMessage).toEqual('No errors')
      expect(resp).toHaveProperty('commit_hash')
    } finally {
      await sim.close()
    }
  })
})

describe.each(models)('Standard [%s] - pubkey', function (m) {
  test.each(cartesianProduct(curves, [APP_DERIVATION, "m/44'/1729'"]))(
    'get pubkey and addr %s, %s',
    async function (curve, derivation_path) {
      const sim = new Zemu(m.path)
      try {
        await sim.start({ ...defaultOptions, model: m.name })
        const app = new TezosApp(sim.getTransport())
        const resp = await app.getAddressAndPubKey(derivation_path, curve)

        console.log(resp, m.name)

        expect(resp.returnCode).toEqual(0x9000)
        expect(resp.errorMessage).toEqual('No errors')
        expect(resp).toHaveProperty('publicKey')
        expect(resp).toHaveProperty('address')
        expect(resp.address).toEqual(await app.publicKeyToAddress(resp.publicKey, curve))
        expect(resp.address).toContain('tz')
      } finally {
        await sim.close()
      }
    },
  )
})

describe.each(models)('Standard [%s]; legacy - pubkey', function (m) {
  test.each(cartesianProduct(curves, ["m/44'/1729'"]))(
    'get pubkey and compute addr %s, %s',
    async function (curve, derivation_path) {
      const sim = new Zemu(m.path)
      try {
        await sim.start({ ...defaultOptions, model: m.name })
        const app = new TezosApp(sim.getTransport())
        const resp = await app.legacyGetPubKey(derivation_path, curve)

        console.log(resp, m.name)

        expect(resp.returnCode).toEqual(0x9000)
        expect(resp.errorMessage).toEqual('No errors')
        expect(resp).toHaveProperty('publicKey')
        expect(resp).toHaveProperty('address')
        expect(resp.address).toEqual(await app.publicKeyToAddress(resp.publicKey, curve))
        expect(resp.address).toContain('tz')
      } finally {
        await sim.close()
      }
    },
  )
})

const SIGN_TEST_DATA = cartesianProduct(curves,
                                        [{
                                          name: 'transfer',
                                          nav: { s: [13, 0], x: [11, 0], sp: [11, 0] },
                                          op: SAMPLE_TRANSACTION
                                        },
                                         {
                                          name: 'known baker',
                                          nav: { s: [10, 0], x: [9, 0], sp : [9, 0] },
                                          op: KNOWN_DELEGATE
                                        }])
const MICHELSON_SIGN_TEST_DATA = cartesianProduct(curves,
                                                  [{
                                                    name: 'blind-hello',
                                                    nav: { s: [2, 0], x: [3, 0], sp: [3, 0] },
                                                    op: Buffer.from("hello@zondax.ch")
                                                  }])

describe.each(models)('Standard [%s]; sign', function (m) {
  test.each(SIGN_TEST_DATA)('sign operation', async function (curve, data) {
    const sim = new Zemu(m.path)
    try {
      await sim.start({ ...defaultOptions, model: m.name })
      const app = new TezosApp(sim.getTransport())
      const msg = Buffer.from(data.op.blob, 'hex')
      const respReq = app.signOperation(APP_DERIVATION, curve, msg)

      await sim.waitUntilScreenIsNot(sim.getMainMenuSnapshot(), 20000)

      const navigation = m.name == 'nanox' ? data.nav.x : m.name == "nanosp" ? data.nav.sp : data.nav.s;
      await sim.navigateAndCompareSnapshots('.', `${m.prefix.toLowerCase()}-sign-${data.name}-${curve}`, navigation)

      const resp = await respReq

      console.log(resp, m.name, data.name, curve)

      expect(resp.returnCode).toEqual(0x9000)
      expect(resp.errorMessage).toEqual('No errors')
      expect(resp).toHaveProperty('hash')
      expect(resp).toHaveProperty('signature')
      expect(resp.hash).toEqual(await app.sig_hash(msg, 'operation'))

      const resp_addr = await app.getAddressAndPubKey(APP_DERIVATION, curve)

      let signatureOK = true
      switch (curve) {
        case Curve.Ed25519:
        case Curve.Ed25519_Slip10:
          signatureOK = ed25519.verify(resp.signature, resp.hash, resp_addr.publicKey.slice(1, 33))
          break

        case Curve.Secp256K1:
          resp.signature[0] = 0x30
          signatureOK = secp256k1.verify(resp.signature, resp.hash, resp_addr.publicKey)
          break

        case Curve.Secp256R1:
          // sig = sepc256k1.importsignature(resp.signature) // From DER to RS?
          // signatureOK = secp256r1.verify(resp.hash, sigRS, resp_addr.publicKey);
          break

        default:
          throw Error('not a valid curve type')
      }
      expect(signatureOK).toEqual(true)
    } finally {
      await sim.close()
    }
  })

  test.each(MICHELSON_SIGN_TEST_DATA)('michelson sign', async function (curve, data) {
    const sim = new Zemu(m.path)
    try {
      await sim.start({ ...defaultOptions, model: m.name })
      const app = new TezosApp(sim.getTransport())
      const msg = data.op;
      const respReq = app.signMichelson(APP_DERIVATION, curve, msg)

      await sim.waitUntilScreenIsNot(sim.getMainMenuSnapshot(), 20000)

      const navigation = m.name == 'nanox' ? data.nav.x : m.name == "nanosp" ? data.nav.sp : data.nav.s;
      await sim.navigateAndCompareSnapshots('.', `${m.prefix.toLowerCase()}-sign-${data.name}-${curve}`, navigation)

      const resp = await respReq

      console.log(resp, m.name, data.name, curve)

      expect(resp.returnCode).toEqual(0x9000)
      expect(resp.errorMessage).toEqual('No errors')
      expect(resp).toHaveProperty('hash')
      expect(resp).toHaveProperty('signature')
      expect(resp.hash).toEqual(await app.sig_hash(msg, 'michelson'))

      const resp_addr = await app.getAddressAndPubKey(APP_DERIVATION, curve)

      let signatureOK = true
      switch (curve) {
        case Curve.Ed25519:
        case Curve.Ed25519_Slip10:
          signatureOK = ed25519.verify(resp.signature, resp.hash, resp_addr.publicKey.slice(1, 33))
          break

        case Curve.Secp256K1:
          resp.signature[0] = 0x30
          signatureOK = secp256k1.verify(resp.signature, resp.hash, resp_addr.publicKey)
          break

        case Curve.Secp256R1:
          // sig = sepc256k1.importsignature(resp.signature) // From DER to RS?
          // signatureOK = secp256r1.verify(resp.hash, sigRS, resp_addr.publicKey);
          break

        default:
          throw Error('not a valid curve type')
      }
      expect(signatureOK).toEqual(true)
    } finally {
      await sim.close()
    }
  })
})

describe.each(models)('Standard [%s]; legacy - sign with hash', function (m) {
  test.each(SIGN_TEST_DATA)('sign operation', async function (curve, data) {
    const sim = new Zemu(m.path)
    try {
      await sim.start({ ...defaultOptions, model: m.name })
      const app = new TezosApp(sim.getTransport())
      const msg = Buffer.from(data.op.blob, 'hex')
      const respReq = app.legacySignWithHash(APP_DERIVATION, curve, msg)

      await sim.waitUntilScreenIsNot(sim.getMainMenuSnapshot(), 20000)

      const navigation = m.name == 'nanox' ? data.nav.x : m.name == "nanosp" ? data.nav.sp : data.nav.s;
      await sim.navigateAndCompareSnapshots('.', `${m.prefix.toLowerCase()}-legacy-sign-with-hash-${data.name}-${curve}`, navigation)

      const resp = await respReq

      console.log(resp, m.name, data.name, curve)

      expect(resp.returnCode).toEqual(0x9000)
      expect(resp.errorMessage).toEqual('No errors')
      expect(resp).toHaveProperty('hash')
      expect(resp).toHaveProperty('signature')
      expect(resp.hash).toEqual(await app.sig_hash(msg, 'operation'))

      const resp_addr = await app.getAddressAndPubKey(APP_DERIVATION, curve)

      let signatureOK = true
      switch (curve) {
        case Curve.Ed25519:
        case Curve.Ed25519_Slip10:
          signatureOK = ed25519.verify(resp.signature, resp.hash, resp_addr.publicKey.slice(1, 33))
          break

        case Curve.Secp256K1:
          resp.signature[0] = 0x30
          signatureOK = secp256k1.verify(resp.signature, resp.hash, resp_addr.publicKey)
          break

        case Curve.Secp256R1:
          // sig = sepc256k1.importsignature(resp.signature) // From DER to RS?
          // signatureOK = secp256r1.verify(resp.hash, sigRS, resp_addr.publicKey);
          break

        default:
          throw Error('not a valid curve type')
      }
      expect(signatureOK).toEqual(true)
    } finally {
      await sim.close()
    }
  })

  test.each(MICHELSON_SIGN_TEST_DATA)('michelson sign', async function (curve, data) {
    const sim = new Zemu(m.path)
    try {
      await sim.start({ ...defaultOptions, model: m.name })
      const app = new TezosApp(sim.getTransport())
      const msg = data.op;
      const respReq = app.legacySignWithHash(APP_DERIVATION, curve, msg, 'michelson')

      await sim.waitUntilScreenIsNot(sim.getMainMenuSnapshot(), 20000)

      const navigation = m.name == 'nanox' ? data.nav.x : m.name == "nanosp" ? data.nav.sp : data.nav.s;
      await sim.navigateAndCompareSnapshots('.', `${m.prefix.toLowerCase()}-sign-${data.name}-${curve}`, navigation)

      const resp = await respReq

      console.log(resp, m.name, data.name, curve)

      expect(resp.returnCode).toEqual(0x9000)
      expect(resp.errorMessage).toEqual('No errors')
      expect(resp).toHaveProperty('hash')
      expect(resp).toHaveProperty('signature')
      expect(resp.hash).toEqual(await app.sig_hash(msg, 'michelson'))

      const resp_addr = await app.getAddressAndPubKey(APP_DERIVATION, curve)

      let signatureOK = true
      switch (curve) {
        case Curve.Ed25519:
        case Curve.Ed25519_Slip10:
          signatureOK = ed25519.verify(resp.signature, resp.hash, resp_addr.publicKey.slice(1, 33))
          break

        case Curve.Secp256K1:
          resp.signature[0] = 0x30
          signatureOK = secp256k1.verify(resp.signature, resp.hash, resp_addr.publicKey)
          break

        case Curve.Secp256R1:
          // sig = sepc256k1.importsignature(resp.signature) // From DER to RS?
          // signatureOK = secp256r1.verify(resp.hash, sigRS, resp_addr.publicKey);
          break

        default:
          throw Error('not a valid curve type')
      }
      expect(signatureOK).toEqual(true)
    } finally {
      await sim.close()
    }
  })
})
