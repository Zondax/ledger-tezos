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

import Zemu, { DeviceModel } from '@zondax/zemu'
import TezosApp, { Curve } from '@zondax/ledger-tezos'
import { APP_DERIVATION, cartesianProduct, curves, defaultOptions } from './common'
import * as secp256k1 from 'noble-secp256k1'

import { SAMPLE_PROPOSALS, SAMPLE_TRANSACTION, SAMPLE_DELEGATION, SAMPLE_ENDORSEMENT, SAMPLE_SEED_NONCE_REVELATION, SAMPLE_BALLOT, SAMPLE_REVEAL } from './tezos'

const ed25519 = require('ed25519-supercop')

const Resolve = require('path').resolve
const APP_PATH_S = Resolve('../rust/app/output/app_s_baking.elf')
const APP_PATH_X = Resolve('../rust/app/output/app_x_baking.elf')

const models: DeviceModel[] = [
  { name: 'nanos', prefix: 'BS', path: APP_PATH_S },
  { name: 'nanox', prefix: 'BX', path: APP_PATH_X },
]

describe.each(models)('Standard baking [%s]', function (m) {
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
      await sim.navigateAndCompareSnapshots('.', `${m.prefix.toLowerCase()}-mainmenu`, [1, 0, 0, 5, -5])
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

      console.log(resp, m.name)

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

describe.each(models)('Standard baking [%s]; legacy', function (m) {
  test('get app version', async function () {
    const sim = new Zemu(m.path)
    try {
      await sim.start({ ...defaultOptions, model: m.name })
      const app = new TezosApp(sim.getTransport())
      const resp = await app.legacyGetVersion()

      console.log(resp, m.name)

      expect(resp.returnCode).toEqual(0x9000)
      expect(resp.errorMessage).toEqual('No errors')
      expect(resp).toHaveProperty('baking')
      expect(resp.baking).toBe(true)
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

      console.log(resp, m.name)
      expect(resp.returnCode).toEqual(0x9000)
      expect(resp.errorMessage).toEqual('No errors')
      expect(resp).toHaveProperty('commit_hash')
    } finally {
      await sim.close()
    }
  })
})

describe.each(models)('Standard baking [%s]; legacy - watermark', function (m) {
  test('reset watermark and verify', async function () {
    const sim = new Zemu(m.path)
    try {
      await sim.start({ ...defaultOptions, model: m.name })
      const app = new TezosApp(sim.getTransport())
      const resp = await app.legacyResetHighWatermark(42)
      console.log(resp)

      expect(resp.returnCode).toEqual(0x9000)
      expect(resp.errorMessage).toEqual('No errors')

      const verify = await app.legacyGetHighWatermark()
      console.log(verify)

      expect(verify.main).toEqual(42)
    } finally {
      await sim.close()
    }
  })

  test('get main high watermark', async function () {
    const sim = new Zemu(m.path)
    try {
      await sim.start({ ...defaultOptions, model: m.name })
      const app = new TezosApp(sim.getTransport())

      //reset watermark to 0 so we can read from the application
      await app.legacyResetHighWatermark(0)

      const resp = await app.legacyGetHighWatermark()

      console.log(resp)

      expect(resp.returnCode).toEqual(0x9000)
      expect(resp.errorMessage).toEqual('No errors')
      expect(resp).toHaveProperty('main')
      expect(resp).toHaveProperty('test')
      expect(resp.test).toBeNull()
      expect(resp).toHaveProperty('chain_id')
      expect(resp.chain_id).toBeNull()
    } finally {
      await sim.close()
    }
  })
})

describe.each(models)('Standard baking [%s] - pubkey', function (m) {
  test.each(cartesianProduct(curves, [true, false]))('get pubkey and addr %s, %s', async function (curve, show) {
    const sim = new Zemu(m.path)
    try {
      await sim.start({ ...defaultOptions, model: m.name })
      const app = new TezosApp(sim.getTransport())
      let resp

      if (show) {
        const respReq = app.showAddressAndPubKey(APP_DERIVATION, curve)

        await sim.waitUntilScreenIsNot(sim.getMainMenuSnapshot(), 20000)

        let steps = 2
        if (m.name == 'nanox') {
          sim.clickRight()
          steps = 1
        }

        await sim.compareSnapshotsAndAccept('.', `${m.prefix.toLowerCase()}-pubkey-${curve}`, steps)
        resp = await respReq
      } else {
        resp = await app.getAddressAndPubKey(APP_DERIVATION, curve)
      }

      console.log(resp, m.name)

      expect(resp.returnCode).toEqual(0x9000)
      expect(resp.errorMessage).toEqual('No errors')
      expect(resp).toHaveProperty('publicKey')
      expect(resp).toHaveProperty('address')
      expect(resp.address).toEqual(app.publicKeyToAddress(resp.publicKey, curve))
      expect(resp.address).toContain('tz')
    } finally {
      await sim.close()
    }
  })
})

describe.each(models)('Standard baking [%s]; legacy - pubkey', function (m) {
  test.each(cartesianProduct(curves, [true, false]))('get pubkey and compute addr %s, %s', async function (curve, show) {
    const sim = new Zemu(m.path)
    try {
      await sim.start({ ...defaultOptions, model: m.name })
      const app = new TezosApp(sim.getTransport())
      let resp

      if (show) {
        const respReq = app.legacyPromptPubKey(APP_DERIVATION, curve)

        await sim.waitUntilScreenIsNot(sim.getMainMenuSnapshot(), 20000)

        let steps = 2
        if (m.name == 'nanox') {
          sim.clickRight()
          steps = 1
        }

        await sim.compareSnapshotsAndAccept('.', `${m.prefix.toLowerCase()}-legacy-pubkey-${curve}`, steps)
        resp = await respReq
      } else {
        resp = await app.legacyGetPubKey(APP_DERIVATION, curve)
      }

      console.log(resp, m.name)

      expect(resp.returnCode).toEqual(0x9000)
      expect(resp.errorMessage).toEqual('No errors')
      expect(resp).toHaveProperty('publicKey')
      expect(resp).toHaveProperty('address')
      expect(resp.address).toEqual(app.publicKeyToAddress(resp.publicKey, curve))
      expect(resp.address).toContain('tz')
    } finally {
      await sim.close()
    }
  })
})

describe.each(models)('Standard baking [%s] - authorize', function (m) {
  test.each(curves)('Authorize baking %s', async function (curve) {
    const sim = new Zemu(m.path)
    try {
      await sim.start({ ...defaultOptions, model: m.name })
      const app = new TezosApp(sim.getTransport())
      const resp = await app.authorizeBaking(APP_DERIVATION, curve)

      console.log(resp, m.name)
      expect(resp.returnCode).toEqual(0x9000)
    } finally {
      await sim.close()
    }
  })

  test.each(curves)('Authorize/Deauthorize full-cycle %s', async function (curve) {
    const sim = new Zemu(m.path)
    try {
      await sim.start({ ...defaultOptions, model: m.name })
      const app = new TezosApp(sim.getTransport())
      const resp = await app.authorizeBaking(APP_DERIVATION, curve)

      console.log(resp, m.name)
      expect(resp.returnCode).toEqual(0x9000)

      const query = await app.queryAuthKeyWithCurve()

      console.log(query, m.name)
      expect(query.returnCode).toEqual(0x9000)
      expect(query.curve).toEqual(curve)

      const query2 = await app.deauthorizeBaking()

      console.log(query2, m.name)
      expect(query2.returnCode).toEqual(0x9000)

      const query3 = await app.queryAuthKeyWithCurve()

      console.log(query3, m.name)
      expect(query3.returnCode).not.toEqual(0x9000)

      const query4 = await app.authorizeBaking(APP_DERIVATION, curve)

      console.log(query4, m.name)
      expect(query4.returnCode).toEqual(0x9000)

      const query5 = await app.queryAuthKeyWithCurve()

      console.log(query5, m.name)
      expect(query5.returnCode).toEqual(0x9000)
      expect(query5.curve).toEqual(curve)
    } finally {
      await sim.close()
    }
  })
})

describe.each(models)('Standard baking [%s] - endorsement, blocklevel', function (m) {
  test.each(curves)('Sign endorsement [%s]', async function (curve) {
    const sim = new Zemu(m.path)
    try {
      await sim.start({ ...defaultOptions, model: m.name })
      const app = new TezosApp(sim.getTransport())
      const resp = await app.authorizeBaking(APP_DERIVATION, curve)

      console.log(resp, m.name)
      expect(resp.returnCode).toEqual(0x9000)

      const baker_blob = app.get_endorsement_info(2, 0, Buffer.alloc(32), 5, 2)

      const respReq = app.signBaker(APP_DERIVATION, curve, baker_blob)

      await sim.waitUntilScreenIsNot(sim.getMainMenuSnapshot(), 20000)
      if (m.name == 'nanox') {
        sim.clickRight()
      }
      await sim.compareSnapshotsAndAccept('.', `${m.prefix.toLowerCase()}-bakersign-endorsement`, 5)

      const respSig = await respReq

      console.log(respSig, m.name)

      expect(respSig.returnCode).toEqual(0x9000)
      expect(respSig.errorMessage).toEqual('No errors')
    } finally {
      await sim.close()
    }
  })

  test.each(curves)('Sign blocklevel [%s]', async function (curve) {
    const sim = new Zemu(m.path)
    try {
      await sim.start({ ...defaultOptions, model: m.name })
      const app = new TezosApp(sim.getTransport())
      const resp = await app.authorizeBaking(APP_DERIVATION, curve)

      console.log(resp, m.name)
      expect(resp.returnCode).toEqual(0x9000)

      const baker_blob = app.get_blocklevel_info(1, 0, 123456, 1)

      const respReq = app.signBaker(APP_DERIVATION, curve, baker_blob)

      await sim.waitUntilScreenIsNot(sim.getMainMenuSnapshot(), 20000)
      if (m.name == 'nanox') {
        sim.clickRight()
      }
      await sim.compareSnapshotsAndAccept('.', `${m.prefix.toLowerCase()}-bakersign-block`, 3)

      const respSig = await respReq

      console.log(respSig, m.name)

      expect(respSig.returnCode).toEqual(0x9000)
      expect(respSig.errorMessage).toEqual('No errors')
    } finally {
      await sim.close()
    }
  })

  test.each(curves)('Sign blocklevel then endorse [%s]', async function (curve) {
    const sim = new Zemu(m.path)
    try {
      await sim.start({ ...defaultOptions, model: m.name })
      const app = new TezosApp(sim.getTransport())
      const resp = await app.authorizeBaking(APP_DERIVATION, curve)

      console.log(resp, m.name)
      expect(resp.returnCode).toEqual(0x9000)

      const baker_blob = app.get_blocklevel_info(1, 0, 5, 1)

      const sigreq = app.signBaker(APP_DERIVATION, curve, baker_blob)
      await sim.waitUntilScreenIsNot(sim.getMainMenuSnapshot(), 20000)
      if (m.name == 'nanox') {
        await sim.clickRight()
      }
      await sim.clickRight()
      await sim.clickRight()
      await sim.clickRight()
      await sim.clickBoth()
      const sig = await sigreq
      console.log(sig, m.name)
      expect(sig.returnCode).toEqual(0x9000)

      //this should fail as the level is equal to previously signed!!
      const baker_blob2 = app.get_blocklevel_info(1, 0, 5, 1)

      const sig2 = await app.signBaker(APP_DERIVATION, curve, baker_blob2)
      console.log(sig2, m.name)
      expect(sig2.returnCode).not.toEqual(0x9000)

      //this should success as the level is equal to previously signed but is endorsement!!
      const baker_blob3 = app.get_endorsement_info(2, 0, Buffer.alloc(32), 5, 5)

      const sigreq3 = app.signBaker(APP_DERIVATION, curve, baker_blob3)
      await sim.waitUntilScreenIsNot(sim.getMainMenuSnapshot(), 20000)

      if (m.name == 'nanox') {
        await sim.clickRight()
      }
      await sim.clickRight()
      await sim.clickRight()
      await sim.clickRight()
      await sim.clickRight()
      await sim.clickRight()
      await sim.clickBoth()

      const sig3 = await sigreq3
      console.log(sig3, m.name)
      expect(sig3.returnCode).toEqual(0x9000)
    } finally {
      await sim.close()
    }
  })
})

const SIGN_TEST_DATA = cartesianProduct(curves, [
  { name: 'transfer', nav: { s: [13, 0], x: [11, 0] }, op: SAMPLE_TRANSACTION },
  { name: 'delegation', nav: { s: [11, 0], x: [9, 0] }, op: SAMPLE_DELEGATION },
  { name: 'endorsement', nav: { s: [4, 0], x: [4, 0] }, op: SAMPLE_ENDORSEMENT },
  { name: 'seed-nonce-revelation', nav: { s: [6, 0], x: [5, 0] }, op: SAMPLE_SEED_NONCE_REVELATION },
  { name: 'ballot', nav: { s: [9, 0], x: [7, 0] }, op: SAMPLE_BALLOT },
  { name: 'reveal', nav: { s: [11, 0], x: [10, 0] }, op: SAMPLE_REVEAL },
  { name: 'proposals', nav: { s: [10, 0], x: [7, 0] }, op: SAMPLE_PROPOSALS },
])

describe.each(models)('Standard baking [%s] - sign operation', function (m) {
  test.each(SIGN_TEST_DATA)('sign $1.name', async function (curve, data) {
    const sim = new Zemu(m.path)
    try {
      await sim.start({ ...defaultOptions, model: m.name })
      const app = new TezosApp(sim.getTransport())
      const msg = Buffer.from(data.op.blob, 'hex')
      const respReq = app.sign(APP_DERIVATION, curve, msg)

      await sim.waitUntilScreenIsNot(sim.getMainMenuSnapshot(), 200000)

      const navigation = m.name == 'nanox' ? data.nav.x : data.nav.s
      await sim.navigateAndCompareSnapshots('.', `${m.prefix.toLowerCase()}-sign-${data.name}-${curve}`, navigation)

      const resp = await respReq

      console.log(resp, m.name)

      expect(resp.returnCode).toEqual(0x9000)
      expect(resp.errorMessage).toEqual('No errors')
      expect(resp).toHaveProperty('hash')
      expect(resp).toHaveProperty('signature')
      expect(resp.hash).toEqual(app.sig_hash(msg))

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

describe.each(models)('Standard baking [%s]; legacy - sign op with hash', function (m) {
  test.each(SIGN_TEST_DATA)('sign $1.name', async function (curve, data) {
    const sim = new Zemu(m.path)
    try {
      await sim.start({ ...defaultOptions, model: m.name })
      const app = new TezosApp(sim.getTransport())
      const msg = Buffer.from(data.op.blob, 'hex')
      const respReq = app.legacySignWithHash(APP_DERIVATION, curve, msg)

      await sim.waitUntilScreenIsNot(sim.getMainMenuSnapshot(), 20000)

      const navigation = m.name == 'nanox' ? data.nav.x : data.nav.s
      await sim.navigateAndCompareSnapshots('.', `${m.prefix.toLowerCase()}-legacy-sign-with-hash-${data.name}-${curve}`, navigation)

      const resp = await respReq

      console.log(resp, m.name)

      expect(resp.returnCode).toEqual(0x9000)
      expect(resp.errorMessage).toEqual('No errors')
      expect(resp).toHaveProperty('hash')
      expect(resp).toHaveProperty('signature')
      expect(resp.hash).toEqual(app.sig_hash(msg))

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
          // FIXME: add later
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
