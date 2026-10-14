using System.Reflection;
using OpenUnity.Interop;

namespace UnityEngine;

internal sealed partial class LifecycleDispatcher
{
    internal void PumpMutations() => Pump(0);

    private void Pump(int mode)
    {
        CheckThread();
        pumpDepth++;
        Exception? failure = null;
        try
        {
            DrainCallbacks(mode);
        }
        catch (Exception error)
        {
            failure = error;
        }
        pumpDepth--;
        if (pumpDepth == 0)
        {
            try
            {
                ReleaseDeadAttachments();
            }
            catch (Exception cleanup)
            {
                failure = failure is null ? cleanup :
                    new AggregateException("Lifecycle dispatch and attachment cleanup failed.", failure, cleanup);
            }
        }
        if (failure is not null)
            System.Runtime.ExceptionServices.ExceptionDispatchInfo.Capture(failure).Throw();
    }

    private void DrainCallbacks(int mode)
    {
        while (true)
        {
            long token = 0;
            long identity = 0;
            int lifecycleEvent = 0;
            int status = context.Invoke(runtime => mode == 2
                ? OpenUnityNative.ou_time_next(runtime, 1, out token, out identity, out lifecycleEvent)
                : OpenUnityNative.ou_lifecycle_next(runtime, mode, out token, out identity, out lifecycleEvent));
            if (status == 0)
                return;
            RequireSuccess(status == 1 ? 0 : status, "lifecycle event poll");
            DeliverCallback(mode, token, identity, lifecycleEvent);
        }
    }

    private void DeliverCallback(int mode, long token, long identity, int lifecycleEvent)
    {
        try
        {
            if (!attachments.TryGetValue(identity, out MonoBehaviour? script))
                throw new InvalidOperationException("OpenUnity returned an invalid managed lifecycle snapshot.");

            MethodInfo? callback;
            if (lifecycleEvent == 8 && mode == 2)
            {
                callback = fixedCallbacks[script.GetType()];
            }
            else if (lifecycleEvent != 8 && EventNames.ContainsKey(lifecycleEvent) &&
                callbacks[script.GetType()].TryGetValue(lifecycleEvent, out MethodInfo? ordinaryCallback))
            {
                callback = ordinaryCallback;
            }
            else
            {
                throw new InvalidOperationException("OpenUnity returned an invalid managed lifecycle event.");
            }

            callback?.Invoke(script, null);
        }
        catch (Exception error)
        {
            Exception failure = error is TargetInvocationException { InnerException: not null } invoked
                ? invoked.InnerException! : error;
            try
            {
                int status = context.Invoke(runtime => OpenUnityNative.ou_lifecycle_ack(runtime, token, -6));
                if (status != 0 && status != -14)
                    throw RuntimeContext.Failure("failed callback acknowledgement", status);
            }
            catch (Exception cleanup)
            {
                throw new AggregateException("Gameplay callback and token cleanup failed.", failure, cleanup);
            }
            throw new InvalidOperationException("OpenUnity gameplay callback failed; dispatch is faulted.", failure);
        }
        RequireSuccess(context.Invoke(runtime => OpenUnityNative.ou_lifecycle_ack(runtime, token, 0)),
            "lifecycle event acknowledgement");
    }
}
