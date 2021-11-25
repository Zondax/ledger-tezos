import Zemu from '@zondax/zemu'
import TezosApp from '@zondax/ledger-tezos'
import path from 'path'
import { readFileSync } from 'fs'

import { fileURLToPath } from 'url'
import { dirname } from 'path'

const __filename = fileURLToPath(import.meta.url)
const __dirname = dirname(__filename)

const APP_PATH = path.resolve('../rust/app/output/app_s.elf')
const CLA = 0x80
const APP_DERIVATION = "m/44'/1729'/0'/0'"

const seed = 'equip will roof matter pink blind book anxiety banner elbow sun young'
const SIM_OPTIONS = {
  logging: true,
  start_delay: 4000,
  X11: true,
  custom: `-s "${seed}" --color LAGOON_BLUE`,
  model: 'nanos',
  startText: 'DO NOT USE',
}

const TEST_VECTOR_DELEGATION = JSON.parse(readFileSync(__dirname + '/../test-vectors/delegation.json', 'utf-8'))
const KNOWN_DELEGATE = TEST_VECTOR_DELEGATION[0]

async function beforeStart() {
  process.on('SIGINT', () => {
    Zemu.default.stopAllEmuContainers(function () {
      process.exit()
    })
  })
  await Zemu.default.checkAndPullImage()
}

async function beforeEnd() {
  await Zemu.default.stopAllEmuContainers()
}

async function debugScenario1(sim, app) {
  // Here you can customize what you want to do :)
}

async function callTestFunction(sim, app) {
  const msg = Buffer.from(KNOWN_DELEGATE.blob, 'hex')
  let responseReq = app.signOperation(APP_DERIVATION, 3, msg)

  await sim.waitUntilScreenIsNot(sim.getMainMenuSnapshot(), 2000000)

  for (let i = 0; i < 10; i++) {
    await sim.clickRight();
  }

  await sim.clickBoth();

  const resp = await responseReq

  console.log(resp)
}

async function main() {
  await beforeStart()

  SIM_OPTIONS['custom'] = SIM_OPTIONS['custom'] + ' --debug'

  const sim = new Zemu.default(APP_PATH)

  try {
    await sim.start(SIM_OPTIONS)
    const app = new TezosApp.default(sim.getTransport())

    ////////////
    /// TIP you can use zemu commands here to take the app to the point where you trigger a breakpoint

    await callTestFunction(sim, app)

    /// TIP
  } finally {
    await sim.close()
    await beforeEnd()
  }
}

;(async () => {
  await main()
})()
