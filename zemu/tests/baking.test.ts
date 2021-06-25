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

import Zemu, {DeviceModel} from "@zondax/zemu";
import TezosApp, {Curve} from "@zondax/ledger-tezos";
import { APP_DERIVATION, defaultOptions, curves, cartesianProduct } from './common'
import * as secp from "noble-secp256k1"
const ed25519 = require('ed25519-supercop')

const Resolve = require("path").resolve;
const APP_PATH_S = Resolve("../rust/app/output/app_s_baking.elf");
const APP_PATH_X = Resolve("../rust/app/output/app_x_baking.elf");

const models: DeviceModel[] = [
    {name: 'nanos', prefix: 'BS', path: APP_PATH_S},
    {name: 'nanox', prefix: 'BX', path: APP_PATH_X},
]

jest.setTimeout(60000)

describe.each(models)('Standard baking [%s]', function (m) {
    test('can start and stop container', async function () {
        const sim = new Zemu(m.path);
        try {
            await sim.start({...defaultOptions, model: m.name});
        } finally {
            await sim.close();
        }
    });

    test('main menu', async function () {
        const sim = new Zemu(m.path);
        try {
            await sim.start({...defaultOptions, model: m.name});
            await sim.navigateAndCompareSnapshots('.', `${m.prefix.toLowerCase()}-mainmenu`, [1, 0, 0, 5, -5])
        } finally {
            await sim.close();
        }
    });

    test('get app version', async function () {
        const sim = new Zemu(m.path);
        try {
            await sim.start({...defaultOptions, model: m.name,});
            const app = new TezosApp(sim.getTransport());
            const resp = await app.getVersion();

            console.log(resp, m.name);

            expect(resp.returnCode).toEqual(0x9000);
            expect(resp.errorMessage).toEqual("No errors");
            expect(resp).toHaveProperty("testMode")
            expect(resp).toHaveProperty("major");
            expect(resp).toHaveProperty("minor");
            expect(resp).toHaveProperty("patch");
        } finally {
            await sim.close();
        }
    });

})

describe.each(models)('Standard baking [%s]; legacy', function (m) {
    test('get app version', async function () {
        const sim = new Zemu(m.path);
        try {
            await sim.start({...defaultOptions, model: m.name,});
            const app = new TezosApp(sim.getTransport());
            const resp = await app.legacyGetVersion();

            console.log(resp, m.name);

            expect(resp.returnCode).toEqual(0x9000);
            expect(resp.errorMessage).toEqual("No errors");
            expect(resp).toHaveProperty("baking")
            expect(resp.baking).toBe(true);
            expect(resp).toHaveProperty("major");
            expect(resp).toHaveProperty("minor");
            expect(resp).toHaveProperty("patch");
        } finally {
            await sim.close();
        }
    });

    test('get git app', async function() {
        const sim = new Zemu(m.path);
        try {
            await sim.start({...defaultOptions, model: m.name});
            const app = new TezosApp(sim.getTransport());
            const resp = await app.legacyGetGit();

            console.log(resp, m.name);
            expect(resp.returnCode).toEqual(0x9000);
            expect(resp.errorMessage).toEqual("No errors");
            expect(resp).toHaveProperty("commit_hash");
        } finally {
            await sim.close();
        }
    });
})

describe.each(models)('Standard baking [%s]; legacy - watermark', function (m) {
    test('reset watermark and verify', async function () {
        const sim = new Zemu(m.path);
        try {
            await sim.start({...defaultOptions, model: m.name,});
            const app = new TezosApp(sim.getTransport());
            const resp = await app.legacyResetHighWatermark(42);
            console.log(resp);

            expect(resp.returnCode).toEqual(0x9000);
            expect(resp.errorMessage).toEqual("No errors");

            const verify = await app.legacyGetHighWatermark();
            console.log(verify);

            expect(verify.main).toEqual(42);
        } finally {
            await sim.close();
        }
    });

    test('get main high watermark', async function () {
        const sim = new Zemu(m.path);
        try {
            await sim.start({...defaultOptions, model: m.name,});
            const app = new TezosApp(sim.getTransport());

            //reset watermark to 0 so we can read from the application
            await app.legacyResetHighWatermark(0);

            const resp = await app.legacyGetHighWatermark();

            console.log(resp);

            expect(resp.returnCode).toEqual(0x9000);
            expect(resp.errorMessage).toEqual("No errors");
            expect(resp).toHaveProperty("main");
            expect(resp).toHaveProperty("test");
            expect(resp.test).toBeNull();
            expect(resp).toHaveProperty("chain_id");
            expect(resp.chain_id).toBeNull();
        } finally {
            await sim.close();
        }
    });
})

describe.each(models)('Standard baking [%s] - pubkey', function (m) {
    test.each(cartesianProduct(curves, [true, false]))
    ('get pubkey and addr %s, %s', async function(curve, show) {
        const sim = new Zemu(m.path);
        try {
            await sim.start({...defaultOptions, model: m.name});
            const app = new TezosApp(sim.getTransport());
            let resp

            if (show) {
                const respReq = app.showAddressAndPubKey(APP_DERIVATION, curve)

                await sim.waitUntilScreenIsNot(sim.getMainMenuSnapshot(), 20000)

                let steps = 2;
                if (m.name == 'nanox') {
                    sim.clickRight()
                    steps = 1;
                }

                await sim.compareSnapshotsAndAccept('.', `${m.prefix.toLowerCase()}-pubkey-${curve}`, steps)
                resp = await respReq
            } else {
                resp = await app.getAddressAndPubKey(APP_DERIVATION, curve)
            }

            console.log(resp, m.name)

            expect(resp.returnCode).toEqual(0x9000);
            expect(resp.errorMessage).toEqual("No errors");
            expect(resp).toHaveProperty("publicKey");
            expect(resp).toHaveProperty("address");
            expect(resp.address).toEqual(app.publicKeyToAddress(resp.publicKey, curve));
            expect(resp.address).toContain("tz");

        }finally {
            await sim.close();
        }
    });
})

describe.each(models)('Standard baking [%s]; legacy - pubkey', function (m) {
  test.each(cartesianProduct(curves, [true, false]))
  ('get pubkey and compute addr %s, %s', async function (curve, show) {
    const sim = new Zemu(m.path)
    try {
      await sim.start({ ...defaultOptions, model: m.name })
      const app = new TezosApp(sim.getTransport())
      let resp

      if (show) {
        const respReq = app.legacyPromptPubKey(APP_DERIVATION, curve)

        await sim.waitUntilScreenIsNot(sim.getMainMenuSnapshot(), 20000)

        let steps = 2;
        if (m.name == 'nanox') {
          sim.clickRight()
          steps = 1;
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

describe.each(models)('Standard baking [%s]; sign', function (m) {
  test.each(
      cartesianProduct(
          curves,
          [
              Buffer.from("francesco@zondax.ch"),
              Buffer.alloc(300, 0)
          ]))
    ('sign message', async function (curve, msg) {
    const sim = new Zemu(m.path)
    try {
      await sim.start({ ...defaultOptions, model: m.name })
      const app = new TezosApp(sim.getTransport())

      const respReq = app.sign(APP_DERIVATION, curve, msg);

      await sim.waitUntilScreenIsNot(sim.getMainMenuSnapshot(), 20000);
      if (m.name == "nanox") {
          sim.clickRight();
      }
      await sim.compareSnapshotsAndAccept('.', `${m.prefix.toLowerCase()}-sign-${msg.length}`, 2);

      const resp = await respReq;

      console.log(resp, m.name)

      expect(resp.returnCode).toEqual(0x9000)
      expect(resp.errorMessage).toEqual('No errors')
      expect(resp).toHaveProperty('hash')
      expect(resp).toHaveProperty('signature')
      expect(resp.hash).toEqual(app.sig_hash(msg))

        const resp_addr = await app.getAddressAndPubKey(APP_DERIVATION, curve);

        let signatureOK = true;
        switch (curve) {
            case Curve.Ed25519:
            case Curve.Ed25519_Slip10:
                signatureOK = ed25519.verify(resp.signature, resp.hash, resp_addr.publicKey.slice(1,33))
                break;

            case Curve.Secp256K1:
                signatureOK = secp.verify(resp.signature, resp.hash, resp_addr.publicKey);
                break;

            case Curve.Secp256R1:
                break;

            default:
                throw Error("not a valid curve type")
        }
        expect(signatureOK).toEqual(true);

    } finally {
      await sim.close()
    }
  })
})
