module queue;

public struct QueueHeader {
  public uint readCounter;
  public uint writeCounter;
}

public void queueReset(
  RWStructuredBuffer<QueueHeader> qh
) {
  qh[0].readCounter = 0;
  qh[0].writeCounter = 0;
}

public int queuePush(
  RWStructuredBuffer<QueueHeader> qh,
  RWStructuredBuffer<uint> qd,
  uint value
) {
  uint index;
  InterlockedAdd(qh[0].writeCounter, 1, index);

  qd[index] = value;
  return index;
}

public int queueRead(
  RWStructuredBuffer<QueueHeader> qh,
  RWStructuredBuffer<uint> qd,
  out uint outVal
) {
  uint index;
  InterlockedAdd(qh[0].readCounter, 1, index);
  if (index > qh[0].writeCounter) {
    return -1;
  } else {
    outVal = qd[index];
    return index;
  }
}
