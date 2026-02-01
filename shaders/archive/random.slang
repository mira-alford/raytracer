module random;

// Based on this: https://math.stackexchange.com/a/340028
public struct RandomResult {
  public uint4 state;
  public float value;
};

public uint TausStep(uint z, int S1, int S2, int S3, uint M) {
  uint b = (((z << S1) ^ z) >> S2);
  return (((z & M) << S3) ^ b);
}

public uint LCGStep(uint z, uint A, uint C) {
  return (A * z + C);
}

// Random value is between 0 and 1
public RandomResult Random(uint4 state) {
  state.x = TausStep(state.x, 13, 19, 12, 4294967294);
  state.y = TausStep(state.y, 2, 25, 4, 4294967288);
  state.z = TausStep(state.z, 3, 11, 17, 4294967280);
  state.w = LCGStep(state.w, 1664525, 1013904223);

  RandomResult result;
  result.state = state;
  result.value = 2.3283064365387e-10 * (state.x ^ state.y ^ state.z ^ state.w);

  return result;
}

public float random_gen(RWStructuredBuffer<uint4> rs, uint idx) {
  RandomResult r1 = Random(rs[idx]);
  rs[idx] = r1.state;
  return r1.value;
}
