module.exports = {
  preset: 'ts-jest',
  testEnvironment: 'node',
  transformIgnorePatterns: ['^.+\\.js$'],
  globalSetup: "./globalsetup.ts",
  globalTeardown: "./globalteardown.ts"
}
