using OpenUnity.Interop;

namespace UnityEngine;

internal sealed partial class LifecycleDispatcher
{
    internal int FrameCount
    {
        get
        {
            CheckThread();
            int count = context.Invoke(OpenUnityNative.ou_lifecycle_frame_count);
            Object.CheckNative(count, "frame count query");
            return count;
        }
    }

    internal int Tick()
    {
        CheckThread();
        if (pumpDepth != 0)
            throw RuntimeContext.Failure("reentrant frame begin", -13);
        RequireSuccess(context.Invoke(OpenUnityNative.ou_lifecycle_begin_frame), "frame begin");
        for (int phase = 1; phase <= 5; phase++)
        {
            RequireSuccess(context.Invoke(runtime => OpenUnityNative.ou_lifecycle_begin_phase(runtime, phase)), "frame phase begin");
            Pump(1);
        }
        return FrameCount;
    }

    private static void RequireTimeCapability()
    {
        uint version = OpenUnityNative.ou_time_abi_version();
        if (version != OpenUnityNative.TimeAbiVersion)
            throw new NotSupportedException($"OpenUnity Time ABI 1.0 is required; received 0x{version:X8}.");
    }

    internal (double[] Values, bool InFixedTimeStep) ReadTime()
    {
        CheckThread();
        RequireTimeCapability();
        double[] values = new double[6];
        int frame = 0;
        int fixedContext = 0;
        RequireSuccess(context.Invoke(runtime => OpenUnityNative.ou_time_snapshot(
            runtime, values, values.Length, out frame, out fixedContext)), "Time snapshot");
        if (fixedContext is not (0 or 1))
            throw new InvalidOperationException("OpenUnity returned an invalid Time context flag.");
        return (values, fixedContext == 1);
    }

    internal void SetTime(int property, float value)
    {
        CheckThread();
        RequireTimeCapability();
        RequireSuccess(context.Invoke(runtime => OpenUnityNative.ou_time_set(runtime, property, value)), "Time setting");
    }

    internal void ConfigureTime(int source, double step)
    {
        CheckThread();
        RequireTimeCapability();
        RequireSuccess(context.Invoke(runtime => OpenUnityNative.ou_time_configure(runtime, source, step)), "Time configuration");
        timed = true;
    }

    internal int TickTimed()
    {
        CheckThread();
        if (pumpDepth != 0)
            throw RuntimeContext.Failure("reentrant frame dispatch", -13);
        RequireSuccess(context.Invoke(OpenUnityNative.ou_time_begin_frame), "timed frame begin");
        BeginTimedPhase(1);
        while (true)
        {
            int status = context.Invoke(OpenUnityNative.ou_time_begin_fixed);
            if (status == 0)
                break;
            RequireSuccess(status == 1 ? 0 : status, "fixed iteration begin");
            Pump(2);
        }
        for (int phase = 2; phase <= 5; phase++)
            BeginTimedPhase(phase);
        return FrameCount;
    }

    private void BeginTimedPhase(int phase)
    {
        RequireSuccess(context.Invoke(runtime => OpenUnityNative.ou_lifecycle_begin_phase(runtime, phase)),
            "timed frame phase begin");
        Pump(2);
    }

    internal void FlushDestroy(bool timed = false)
    {
        CheckThread();
        if (pumpDepth != 0)
            throw RuntimeContext.Failure("reentrant destruction boundary", -13);
        RequireSuccess(context.Invoke(OpenUnityNative.ou_lifecycle_flush_destroy), "destruction boundary");
        Pump(timed ? 2 : 1);
    }
}
