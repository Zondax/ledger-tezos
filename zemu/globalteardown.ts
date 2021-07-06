import Zemu from '@zondax/zemu'

module.exports = async () => {
  console.log("Executing clean up tasks after finishing all test suites")
  await Zemu.stopAllEmuContainers()
}
