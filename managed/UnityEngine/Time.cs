namespace UnityEngine;

public class Time
{
    private Time() { }

    public static int frameCount => RuntimeContext.Shared.Lifecycle.FrameCount;
    public static bool inFixedTimeStep => RuntimeContext.Shared.Lifecycle.ReadTime().InFixedTimeStep;
    public static float timeScale
    {
        get => (float)RuntimeContext.Shared.Lifecycle.ReadTime().Values[0];
        set => RuntimeContext.Shared.Lifecycle.SetTime(1, value);
    }
    public static float deltaTime => (float)RuntimeContext.Shared.Lifecycle.ReadTime().Values[1];
    public static float fixedDeltaTime
    {
        get => (float)RuntimeContext.Shared.Lifecycle.ReadTime().Values[2];
        set => RuntimeContext.Shared.Lifecycle.SetTime(2, value);
    }
    public static float captureDeltaTime
    {
        get => (float)RuntimeContext.Shared.Lifecycle.ReadTime().Values[3];
        set => RuntimeContext.Shared.Lifecycle.SetTime(3, value);
    }
    public static double timeAsDouble => RuntimeContext.Shared.Lifecycle.ReadTime().Values[4];
    public static double fixedTimeAsDouble => RuntimeContext.Shared.Lifecycle.ReadTime().Values[5];
}
