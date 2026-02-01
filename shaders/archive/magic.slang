import scene_bindings;

[shader("compute")]
[numthreads(8,8,1)]
void computeMain(uint3 threadId : SV_DispatchThreadID) {
  // output[threadId.x + threadId.y * 512] = uint(vertices[0].position.x);
}
