export function get_endorsement_info(chain_id: number, branch: Buffer, level: number, type: 'emmy' | 'endorsement' | 'preendorsement', round?: number): Buffer {
  const result = Buffer.alloc(100) //should be enough for what we are writing
  let offset = 0;

  offset = result.writeUInt32BE(chain_id, offset)
  offset = offset + branch.copy(result, offset)

  switch (type) {
    case 'emmy':
      offset = result.writeUInt8(0, offset) //tag
      offset = result.writeUInt32BE(level, offset)
      return result.subarray(0, offset);
    case 'preendorsement':
      offset = result.writeUInt8(20, offset) //tag
      break;
    case 'endorsement':
      offset = result.writeUInt8(21, offset) //tag
      break;
    default:
      throw new Error("invalid endorsement type")
  }

  offset = result.writeUInt16BE(0, offset) //slot
  offset = result.writeUInt32BE(level, offset);
  offset = result.writeUInt32BE(round!, offset);
  offset = offset + Buffer.alloc(32, 0).copy(result, offset); //block_payload_hash

  return result.subarray(0, offset)
}

export function get_blocklevel_info(chain_id: number, level: number, round?: number): Buffer {
  const result = Buffer.allocUnsafe(100); //should be enough for what we are writing
  let offset = 0;

  offset = result.writeUInt32BE(chain_id, offset)
  offset = result.writeUInt32BE(level, offset)
  offset = result.writeUInt8(42, offset) //proto
  offset = offset + Buffer.alloc(32, 0).copy(result, offset) //predecessor
  offset = result.writeBigUint64BE(BigInt(0), offset); //timestamp
  offset = result.writeUInt8(0, offset); //validation pass
  offset = offset + Buffer.alloc(32, 0).copy(result, offset) //operation hash

  let fitness;
  if (round) {
    //write tenderbake protocol (2)
    //and allocate 4 more bytes for the round
    fitness = Buffer.alloc(5, 2)
    fitness.writeUInt32BE(round!, 1)
  } else {
    fitness = Buffer.alloc(1, 1) //emmy protocol 5 to 11
  };
  offset = result.writeUInt32BE(fitness.length, offset);
  offset = offset + Buffer.alloc(4, 0).copy(result, offset) //fitness padding
  offset = offset + fitness.copy(result, offset)

  return result.subarray(0, offset)
}
