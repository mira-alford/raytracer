module sample;

[Flags]
public enum SampleFlag : uint {
  Converged, // = 1
};

public struct SampleData {
  public float2 screen_pos; // Screen position in 0.0..=1.0
  public uint2 out_pos; // Screen position in pixels
  public uint samples; // Number of samples taken
  public SampleFlag flags; 
};

