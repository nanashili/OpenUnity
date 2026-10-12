namespace UnityEngine;

public partial class Physics
{
    public static SimulationMode simulationMode
    {
        get => RuntimeContext.Shared.Invoke(runtime =>
        {
            PhysicsMotionAccess.RequireCapability();
            return PhysicsMotionAccess.ReadMode(runtime);
        });
        set => RuntimeContext.Shared.Invoke(runtime =>
        {
            PhysicsMotionAccess.RequireCapability();
            int nativeMode = value switch
            {
                SimulationMode.FixedUpdate => 0,
                SimulationMode.Script => 1,
                _ => throw new NotSupportedException(
                    $"OpenUnity Physics simulation mode '{value}' is unsupported.")
            };

            PhysicsMotionAccess.WriteMode(runtime, nativeMode);
            return 0;
        });
    }

    [Obsolete("Physics.autoSimulation has been replaced by Physics.simulationMode", false)]
    public static bool autoSimulation
    {
        get => RuntimeContext.Shared.Invoke(runtime =>
        {
            PhysicsMotionAccess.RequireCapability();
            return PhysicsMotionAccess.ReadMode(runtime) == SimulationMode.FixedUpdate;
        });
        set => RuntimeContext.Shared.Invoke(runtime =>
        {
            PhysicsMotionAccess.RequireCapability();
            return RejectAutoSimulationAssignment();
        });
    }

    public static void Simulate(float step)
    {
        RuntimeContext.Shared.Invoke(runtime =>
        {
            PhysicsMotionAccess.RequireCapability();
            ValidateManualStep(step);
            RequireScriptMode(runtime);
            PhysicsMotionAccess.Simulate(runtime, step);
            return 0;
        });
    }

    public static void SyncTransforms()
    {
        RuntimeContext.Shared.Invoke(runtime =>
        {
            PhysicsMotionAccess.RequireCapability();
            PhysicsMotionAccess.SyncTransforms(runtime);
            return 0;
        });
    }

    private static void ValidateManualStep(float step)
    {
        if (!float.IsFinite(step) || step <= 0.0f)
            throw new ArgumentOutOfRangeException(nameof(step), "Physics simulation step must be finite and positive.");
    }

    private static void RequireScriptMode(long runtime)
    {
        if (PhysicsMotionAccess.ReadMode(runtime) != SimulationMode.Script)
            throw new NotSupportedException(
                "OpenUnity Physics.Simulate requires Physics.simulationMode to be Script.");
    }

    private static int RejectAutoSimulationAssignment() =>
        throw new NotSupportedException(
            "OpenUnity Physics.autoSimulation assignment is unsupported; use Physics.simulationMode.");
}
