module bvh;

public struct AABB {
  public float4 lb;
  public float4 ub;
}

public struct BVHNode {
  public float4 lb;
  public float4 ub;
  public uint left;
  public uint right;
  public uint is_leaf;
  public uint start;
  public uint end;
}
