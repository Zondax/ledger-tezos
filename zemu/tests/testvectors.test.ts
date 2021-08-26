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

const ed25519 = require('ed25519-supercop')

import { TestVector } from '../test-vectors-gen/legacy'

import { readFileSync } from 'fs';

const SAMPLE_OPERATIONS: { blob: Buffer }[] = JSON.parse(readFileSync('/tmp/jest.forged_sample_operations.json', 'utf-8'));
const TEST_VECTORS: TestVector[] = JSON.parse(readFileSync('/tmp/jest.collected_test_vectors.json', 'utf-8'));

describe.each(cartesianProduct(models, curves))('Test Vectors', function (m, curve) {
  console.log(`Going to test {TEST_VECTORS.length} vectors`);
  test.each(TEST_VECTORS)('sign test vector', async function (test_case) {
    console.log(TEST_VECTORS.length)
    const sim = new Zemu(m.path)
    try {
      await sim.start({ ...defaultOptions, model: m.name, pressDelayAfter: 0 })
      const app = new TezosApp(sim.getTransport())

      const msg = Buffer.from(test_case.blob, 'hex')
      const respReq = app.sign(APP_DERIVATION, curve, msg)

      await sim.waitUntilScreenIsNot(sim.getMainMenuSnapshot(), 5000)
      //try to navigate to the end of the transaction
      for (let i = 0; i < 30; i++) {
        await sim.clickRight()
      }

      //and accept, since last screen is reject
      await sim.clickLeft()
      await sim.clickBoth()

      const resp = await respReq
      console.log(test_case, resp, m.name, curve)

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

describe.each(cartesianProduct(models, curves))('Sample Operations', function (m, curve) {
  console.log(`Going to test {SAMPLE_OPERATIONS.length} samples`);
  test.each(SAMPLE_OPERATIONS)('sign sample operation', async function (sample) {
    const sim = new Zemu(m.path)
    try {
      await sim.start({ ...defaultOptions, model: m.name, pressDelayAfter: 0 })
      const app = new TezosApp(sim.getTransport())

      const msg = sample.blob
      const respReq = app.sign(APP_DERIVATION, curve, msg)

      await sim.waitUntilScreenIsNot(sim.getMainMenuSnapshot(), 5000)

      //try to navigate to the end of the transaction
      for (let i = 0; i < 30; i++) {
        await sim.clickRight()
      }

      //and accept, since last screen is reject
      await sim.clickLeft()
      await sim.clickBoth()

      const resp = await respReq
      console.log(sample, resp, m.name, curve)

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
