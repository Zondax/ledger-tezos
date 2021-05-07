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

import Zemu, {DEFAULT_START_OPTIONS, DeviceModel} from "@zondax/zemu";
import TezosApp from "@zondax/ledger-tezos";

const Resolve = require("path").resolve;
const APP_PATH_S = Resolve("../rust/app/output/app_s.elf");
const APP_PATH_X = Resolve("../rust/app/output/app_x.elf");

const APP_SEED = "equip will roof matter pink blind book anxiety banner elbow sun young"

const defaultOptions = {
    ...DEFAULT_START_OPTIONS,
    logging: true,
    custom: `-s "${APP_SEED}"`,
    X11: true,
};

const models: DeviceModel[] = [
    {name: 'nanos', prefix: 'S', path: APP_PATH_S},
    {name: 'nanox', prefix: 'X', path: APP_PATH_X}
]

jest.setTimeout(60000)

describe('Standard', function () {
    test.each(models)('can start and stop container', async function (m) {
        const sim = new Zemu(m.path);
        try {
            await sim.start({...defaultOptions, model: m.name,});
        } finally {
            await sim.close();
        }
    });

    test.each(models)('main menu', async function (m) {
        const sim = new Zemu(m.path);
        try {
            await sim.start({...defaultOptions, model: m.name,});
            expect(await sim.compareSnapshotsAndAccept(".", `${m.prefix.toLowerCase()}-mainmenu`, 3)).toBeTruthy()
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

            console.log(resp);

            expect(resp.returnCode).toEqual(0x9000);
            expect(resp.errorMessage).toEqual("No errors");
            expect(resp).toHaveProperty("testMode");
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
            const resp = await app.getGit();

            console.log(resp);
            expect(resp.returnCode).toEqual(0x9000);
            expect(resp.errorMessage).toEqual("No errors");
            expect(resp).toHaveProperty("commit_hash");
        } finally {
            await sim.close();
        }
    })
});
