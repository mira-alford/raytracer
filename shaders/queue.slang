// queue.slang
//
// Some helper functions for queue maintenance.
module queue;

public void queueReset(
  RWStructuredBuffer<int> qh
) {
  qh[0] = 0;
}

public void queuePush(
  RWStructuredBuffer<int> qh,
  RWStructuredBuffer<uint> qd,
  uint value
) {
  int index;
  InterlockedAdd(qh[0], 1, index);

  qd[index] = value;
}

public int queueRead(
  RWStructuredBuffer<int> qh,
  RWStructuredBuffer<uint> qd,
) {
  int index;
  InterlockedAdd(qh[0], -1, index);
  index = index - 1;
  if (index < 0) {
    InterlockedMax(qh[0], 0);
    return -1;
  } else {
    return int(qd[index]);
  }
}

