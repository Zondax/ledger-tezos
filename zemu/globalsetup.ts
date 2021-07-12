import Zemu from '@zondax/zemu'

const catchExit = async () => {
  process.on('SIGINT', () => {
    Zemu.stopAllEmuContainers()
  })
}

module.exports = async () => {
  console.log("Executing tasks before starting the test suites")

  await catchExit()
  await Zemu.checkAndPullImage()
  await Zemu.stopAllEmuContainers()
}
