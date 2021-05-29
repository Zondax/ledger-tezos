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
import TezosAppDev from "./dev";
import { LedgerError } from "@zondax/ledger-tezos";
import { defaultOptions } from "./common";

const Resolve = require("path").resolve;
const APP_PATH_S = Resolve("../rust/app/output/app_s.elf");
const APP_PATH_BS = Resolve("../rust/app/output/app_s_baking.elf");
const APP_PATH_X = Resolve("../rust/app/output/app_x.elf");
const APP_PATH_BX = Resolve("../rust/app/output/app_x_baking.elf");

const models: DeviceModel[] = [
    {name: 'nanos', prefix: 'S', path: APP_PATH_S},
    {name: 'nanos', prefix: 'BS', path: APP_PATH_BS},
    {name: 'nanox', prefix: 'X', path: APP_PATH_X},
    {name: 'nanox', prefix: 'BX', path: APP_PATH_BX},
]

jest.setTimeout(60000)

function warn_dev(code: LedgerError) {
    if (code === LedgerError.TransactionRejected) {
        console.log("APP might not be built with `dev` feature!");
    }
}

describe('Development specials', function () {
    test.each(models)('catch exception', async function(m) {
        const sim = new Zemu(m.path);
        try {
            const ex = 1; //generic exception
            await sim.start({...defaultOptions, model: m.name});
            const app = new TezosAppDev(sim.getTransport());
            const resp = await app.except(true, ex);

            console.log(resp, m.prefix);
            warn_dev(resp.returnCode);

            expect(resp.returnCode).toEqual(LedgerError.NoErrors);
            expect(resp.errorMessage).toEqual("No errors");
            expect(resp).toHaveProperty("ex");
            expect(resp.ex).toEqual(BigInt(ex));
        } finally {
            await sim.close();
        }
    })

    test.each(models)('throw exception', async function(m) {
        const sim = new Zemu(m.path);
        try {
            const ex = 1; //generic exception
            await sim.start({...defaultOptions, model: m.name});
            const app = new TezosAppDev(sim.getTransport());
            const resp = await app.except(false, ex);

            console.log(resp, m.prefix);
            warn_dev(resp.returnCode);

            expect(resp.returnCode).toEqual(LedgerError.ExecutionError);
            expect(resp.errorMessage).toEqual("Execution Error");
        } finally {
            await sim.close();
        }
    })
});

describe('Unknown exceptions', function () {
    test.each(models)('catch unknown exception', async function(m) {
        const sim = new Zemu(m.path);
        try {
            await sim.start({...defaultOptions, model: m.name});
            const app = new TezosAppDev(sim.getTransport());
            const resp = await app.except(true, 42);

            console.log(resp, m.prefix);
            warn_dev(resp.returnCode);

            if ((resp.returnCode == LedgerError.InvalidP1P2) || (resp.returnCode == LedgerError.NoErrors)) {
                //in case of nanos there's no unknown exception (for now)
            }
        } finally {
            await sim.close();
        }
    })

    test.each(models)('throw unknown exception', async function(m) {
        const sim = new Zemu(m.path);
        try {
            await sim.start({...defaultOptions, model: m.name});
            const app = new TezosAppDev(sim.getTransport());
            const resp = await app.except(false, 42);

            console.log(resp, m.prefix);
            warn_dev(resp.returnCode);

            if ((resp.returnCode == LedgerError.InvalidP1P2) || (resp.returnCode == LedgerError.NoErrors)) {
                //in case of nanos there's no unknown exception (for now)
            }
        } finally {
            await sim.close();
        }
    })
});

describe('SHA256', function () {
    test.each(models)('get hash', async function(m) {
        const sim = new Zemu(m.path);
        try {
            await sim.start({...defaultOptions, model: m.name});
            const app = new TezosAppDev(sim.getTransport());
            const resp = await app.getHash(Buffer.from("francesco@zondax.ch"));

            console.log(resp, m.prefix);
            warn_dev(resp.returnCode);

            expect(resp.returnCode).toEqual(0x9000);
            expect(resp.errorMessage).toEqual("No errors");
            expect(resp).toHaveProperty("hash");
        } finally {
            await sim.close();
        }
    })

    test.each(models)('get long hash', async function(m) {
        const sim = new Zemu(m.path);
        try {
            await sim.start({...defaultOptions, model: m.name});
            const app = new TezosAppDev(sim.getTransport());
            const resp = await app.getHash(Buffer.alloc(300, 0));

            console.log(resp, m.prefix);
            expect(resp.returnCode).toEqual(0x9000);
            expect(resp.errorMessage).toEqual("No errors");
            expect(resp).toHaveProperty("hash");
        } finally {
            await sim.close();
        }
    })
});
