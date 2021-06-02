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
import TezosApp, { Curve } from "@zondax/ledger-tezos";
import { APP_DERIVATION, defaultOptions } from './common'

const Resolve = require("path").resolve;
const APP_PATH_S = Resolve("../rust/app/output/app_s_baking.elf");
const APP_PATH_X = Resolve("../rust/app/output/app_x_baking.elf");

const models: DeviceModel[] = [
    {name: 'nanos', prefix: 'BS', path: APP_PATH_S},
    {name: 'nanox', prefix: 'BX', path: APP_PATH_X},
]

jest.setTimeout(60000)

describe('Standard baking', function () {
    test.each(models)('can start and stop container', async function (m) {
        const sim = new Zemu(m.path);
        try {
            await sim.start({...defaultOptions, model: m.name});
        } finally {
            await sim.close();
        }
    });

    test.each(models)('main menu', async function (m) {
        const sim = new Zemu(m.path);
        try {
            await sim.start({...defaultOptions, model: m.name});
            await sim.navigateAndCompareSnapshots('.', `${m.prefix.toLowerCase()}-mainmenu`, [1, 0, 0, 5, -5])
        } finally {
            await sim.close();
        }
    });

    test.each(models)('get app version', async function (m) {
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

describe('Standard baking; legacy', function () {
    test.each(models)('get app version', async function (m) {
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

    test.each(models)('get git app', async function(m) {
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

describe('Standard baking; legacy - watermark', function () {
    test.each(models)('reset watermark and verify', async function (m) {
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

    test.each(models)('get main high watermark', async function (m) {
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

describe('Standard baking - pubkey', function () {
    test.each(models)('get pubkey and addr ed25519', async function(m) {
        const sim = new Zemu(m.path);
        try {
            await sim.start({...defaultOptions, model: m.name});
            const app = new TezosApp(sim.getTransport());
            const resp = await app.getAddressAndPubKey(APP_DERIVATION, Curve.Ed25519);

            console.log(resp, m.name);

            expect(resp.returnCode).toEqual(0x9000);
            expect(resp.errorMessage).toEqual("No errors");
            expect(resp).toHaveProperty("publicKey");
            expect(resp).toHaveProperty("address");

        }finally {
            await sim.close();
        }
    });

    test.each(models)('get pubkey and addr ed25519 slip10', async function(m) {
        const sim = new Zemu(m.path);
        try {
            await sim.start({...defaultOptions, model: m.name});
            const app = new TezosApp(sim.getTransport());
            const resp = await app.getAddressAndPubKey(APP_DERIVATION, Curve.Ed25519_Slip10);

            console.log(resp, m.name);

            expect(resp.returnCode).toEqual(0x9000);
            expect(resp.errorMessage).toEqual("No errors");
            expect(resp).toHaveProperty("publicKey");
            expect(resp).toHaveProperty("address");

        }finally {
            await sim.close();
        }
    });

    test.each(models)('get pubkey and addr secp256k1', async function(m) {
        const sim = new Zemu(m.path);
        try {
            await sim.start({...defaultOptions, model: m.name});
            const app = new TezosApp(sim.getTransport());
            const resp = await app.getAddressAndPubKey(APP_DERIVATION, Curve.Secp256K1);

            console.log(resp, m.name);

            expect(resp.returnCode).toEqual(0x9000);
            expect(resp.errorMessage).toEqual("No errors");
            expect(resp).toHaveProperty("publicKey");
            expect(resp).toHaveProperty("address");

        }finally {
            await sim.close();
        }
    });

    test.each(models)('get pubkey and addr secp256r1', async function(m) {
        const sim = new Zemu(m.path);
        try {
            await sim.start({...defaultOptions, model: m.name});
            const app = new TezosApp(sim.getTransport());
            const resp = await app.getAddressAndPubKey(APP_DERIVATION, Curve.Secp256R1);

            console.log(resp, m.name);

            expect(resp.returnCode).toEqual(0x9000);
            expect(resp.errorMessage).toEqual("No errors");
            expect(resp).toHaveProperty("publicKey");
            expect(resp).toHaveProperty("address");

        }finally {
            await sim.close();
        }
    });
})
