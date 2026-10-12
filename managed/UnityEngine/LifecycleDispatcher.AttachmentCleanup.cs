namespace UnityEngine;

internal sealed partial class LifecycleDispatcher
{
    private void ReleaseDeadAttachments()
    {
        List<Exception> failures = new();

        TryReleaseDeadScripts(failures);
        TryReleaseDeadOwners(failures);
        TryReleaseDeadNativeComponents(failures);

        ThrowCleanupFailures(failures, "Lifecycle attachment cleanup failed.");
    }

    private void TryReleaseDeadScripts(List<Exception> failures)
    {
        try
        {
            context.Invoke(runtime => ReleaseDeadRootGroup(runtime, attachments, failures));
        }
        catch (Exception failure)
        {
            failures.Add(failure);
        }
    }

    private void TryReleaseDeadOwners(List<Exception> failures)
    {
        try
        {
            context.Invoke(runtime => ReleaseDeadRootGroup(runtime, owners, failures));
        }
        catch (Exception failure)
        {
            failures.Add(failure);
        }
    }

    private void TryReleaseDeadNativeComponents(List<Exception> failures)
    {
        try
        {
            nativeComponents.ReleaseDead(identity =>
                context.Invoke(runtime => IsAlive(runtime, identity)));
        }
        catch (Exception failure)
        {
            failures.Add(failure);
        }
    }

    private static int ReleaseDeadRootGroup<T>(
        long runtime,
        Dictionary<long, T> roots,
        List<Exception> failures)
        where T : class
    {
        foreach (long identity in roots.Keys.ToArray())
        {
            try
            {
                if (!IsAlive(runtime, identity))
                    roots.Remove(identity);
            }
            catch (Exception failure)
            {
                failures.Add(failure);
            }
        }
        return 0;
    }

    private static void ThrowCleanupFailures(List<Exception> failures, string message)
    {
        if (failures.Count == 0)
            return;
        if (failures.Count == 1)
            System.Runtime.ExceptionServices.ExceptionDispatchInfo.Capture(failures[0]).Throw();
        throw new AggregateException(message, failures);
    }

    private static bool IsAlive(long runtime, long identity) =>
        OpenUnity.Interop.OpenUnityNative.ou_object_is_alive(runtime, identity) switch
        {
            0 => false,
            1 => true,
            int status => throw RuntimeContext.Failure("attachment liveness query", status)
        };
}
