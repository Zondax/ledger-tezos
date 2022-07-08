const HARDENED = 0x80000000

export function serializePath(path: string): Buffer {
  if (!path.startsWith('m')) {
    throw new Error('Path should start with "m" (e.g "m/44\'/5757\'/5\'/0/3")')
  }

  const pathArray = path.split('/')

  if (pathArray.length !== 5 && pathArray.length !== 3) {
    throw new Error("Invalid path. (e.g \"m/44'/5757'/5'/0/3\")")
  }

  const buf = Buffer.alloc(1 + (pathArray.length - 1) * 4)
  buf.writeUInt8(pathArray.length - 1) //first byte is the path length

  for (let i = 1; i < pathArray.length; i += 1) {
    let value = 0
    let child = pathArray[i]
    if (child.endsWith("'")) {
      child = child.slice(0, -1)
    }
    value += HARDENED

    const childNumber = Number(child)

    if (Number.isNaN(childNumber)) {
      throw new Error(`Invalid path : ${child} is not a number. (e.g "m/44'/461'/5'/0/3")`)
    }

    if (childNumber >= HARDENED) {
      throw new Error('Incorrect child value (bigger or equal to 0x80000000)')
    }

    value += childNumber

    buf.writeUInt32BE(value, 1 + 4 * (i - 1))
  }

  return buf
}

const createHash = require('crypto').createHash

export function sha256x2(input: Buffer): Buffer {
  const tmp = createHash('sha256').update(input).digest()
  return createHash('sha256').update(tmp).digest()
}
