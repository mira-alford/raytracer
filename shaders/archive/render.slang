module render;

struct VertexInput {
  float3 position;
  float3 texCoords;
}

struct VertexOutput {
  float4 clipPos : SV_Position;
  float2 texCoords;
}

[shader("vertex")]
VertexOutput vertexMain(
  VertexInput model
) {
  VertexOutput outVal;
  outVal.texCoords = model.texCoords.xy;
  outVal.clipPos = float4(model.position, 1.0);
  return outVal;
}

[[vk::binding(0,0)]] Texture2D<float4> tDiffuse;
[[vk::binding(1,0)]] SamplerState sDiffuse;

[shader("fragment")]
float4 fragmentMain(
  VertexOutput input,
) : SV_Target0 {
  return tDiffuse.Sample(sDiffuse, input.texCoords);
}
