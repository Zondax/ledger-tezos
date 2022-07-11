import TransportNodeHid from '@ledgerhq/hw-transport-node-hid'
import TezosApp from '@zondax/ledger-tezos'

async function main() {
  const transport = await TransportNodeHid.default.open();

  const app = new TezosApp.default(transport);

  const version = await app.getVersion();
  console.log(JSON.stringify(version))
}

; (async () => {
  await main()
})()
