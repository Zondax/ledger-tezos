import Zemu from "@zondax/zemu";
import TezosApp from "@zondax/ledger-tezos";
import path from "path";

const APP_PATH = path.resolve("../rust/app/output/app_s.elf");

const seed = "equip will roof matter pink blind book anxiety banner elbow sun young"
const SIM_OPTIONS = {
    logging: true,
    start_delay: 4000,
//    X11: true,
    custom: `-s "${seed}" --color LAGOON_BLUE`,
    model: 'nanos'
};

async function beforeStart() {
    process.on("SIGINT", () => {
        Zemu.default.stopAllEmuContainers(function () {
            process.exit();
        });
    });
    await Zemu.default.checkAndPullImage();
}

async function beforeEnd() {
    await Zemu.default.stopAllEmuContainers();
}

async function debugScenario1(sim, app) {
    // Here you can customize what you want to do :)
}

async function callTestFunction(sim, app) {
    let input = 10;

    let response = await sim.getTransport()
        .send(0x85, 0xFF, 0, 0, Buffer.from([input]), [0x9000, 0x6e00]);

    console.log(response.toString("hex"));
}

async function main() {
    await beforeStart();

    if (process.argv.length > 2 && process.argv[2] === "debug") {
        SIM_OPTIONS["custom"] = SIM_OPTIONS["custom"] + " --debug";
    }

    const sim = new Zemu.default(APP_PATH);

    try {
        await sim.start(SIM_OPTIONS);
        const app = new TezosApp.default(sim.getTransport());

        ////////////
        /// TIP you can use zemu commands here to take the app to the point where you trigger a breakpoint

        await callTestFunction(sim, app);

        /// TIP

    } finally {
        await sim.close();
        await beforeEnd();
    }
}

(async () => {
    await main();
})();
