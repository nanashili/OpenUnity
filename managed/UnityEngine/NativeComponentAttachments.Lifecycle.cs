using System.Collections.Generic;

namespace UnityEngine;

internal sealed partial class NativeComponentAttachments
{
    internal Component? Find(long identity) => committed.GetValueOrDefault(identity);

    internal void ReleaseRetiredOwner(GameObject owner)
    {
        ArgumentNullException.ThrowIfNull(owner);
        if (pendingOwners.ContainsKey(owner.Identity))
            throw new InvalidOperationException("Cannot retire a GameObject with a native component reservation in flight.");
        if (!rootsByOwner.TryGetValue(owner.Identity, out OwnerRoots? roots))
            return;
        if (!ReferenceEquals(roots.Owner, owner))
            throw new InvalidOperationException("A native owner identity is rooted by a different managed owner.");

        List<Exception>? failures = ReleaseRetiredComponents(roots);
        if (roots.Components.Count == 0 && !rootsByOwner.Remove(owner.Identity))
            (failures ??= new()).Add(new InvalidOperationException("The retired native owner root could not be released."));
        ThrowCleanupFailures(failures, "Native component owner cleanup failed.");
    }

    // Native finalization must be observed before calling this method. The
    // predicate is supplied by the dispatcher and may query native liveness.
    internal void ReleaseDead(Func<long, bool> isAlive)
    {
        ArgumentNullException.ThrowIfNull(isAlive);
        List<OwnerRoots> rootsSnapshot = new(rootsByOwner.Values);
        List<Exception>? failures = null;
        foreach (OwnerRoots roots in rootsSnapshot)
        {
            if (pendingOwners.ContainsKey(roots.Owner.Identity))
                continue;
            if (!rootsByOwner.TryGetValue(roots.Owner.Identity, out OwnerRoots? current) ||
                !ReferenceEquals(current, roots))
            {
                (failures ??= new()).Add(new InvalidOperationException(
                    "The native component owner root changed during liveness cleanup."));
                continue;
            }

            List<Exception>? rootFailures = ReleaseDeadComponents(roots, isAlive);
            if (rootFailures is not null)
                (failures ??= new()).AddRange(rootFailures);
        }

        ThrowCleanupFailures(failures, "Native component liveness cleanup failed.");
    }

    private List<Exception>? ReleaseRetiredComponents(OwnerRoots roots)
    {
        List<Exception>? failures = null;
        for (int index = roots.Components.Count - 1; index >= 0; index--)
        {
            Component component = roots.Components[index];
            try
            {
                RemoveCommittedRoot(component);
                roots.Components.RemoveAt(index);
            }
            catch (Exception failure)
            {
                (failures ??= new()).Add(failure);
            }
        }
        return failures;
    }

    private List<Exception>? ReleaseDeadComponents(OwnerRoots roots, Func<long, bool> isAlive)
    {
        List<Exception>? failures = null;
        for (int index = roots.Components.Count - 1; index >= 0; index--)
        {
            Component component = roots.Components[index];
            bool alive;
            try
            {
                alive = isAlive(component.Identity);
            }
            catch (Exception failure)
            {
                (failures ??= new()).Add(failure);
                continue;
            }

            if (alive)
                continue;

            try
            {
                RemoveCommittedRoot(component);
                roots.Components.RemoveAt(index);
            }
            catch (Exception failure)
            {
                (failures ??= new()).Add(failure);
            }
        }

        if (roots.Components.Count == 0 && !rootsByOwner.Remove(roots.Owner.Identity))
            (failures ??= new()).Add(new InvalidOperationException("The dead native owner root could not be released."));
        return failures;
    }

    private void RemoveCommittedRoot(Component component)
    {
        if (!committed.TryGetValue(component.Identity, out Component? existing))
            throw new InvalidOperationException("The canonical native component root is missing.");
        if (!ReferenceEquals(existing, component))
            throw new InvalidOperationException("A native component identity is rooted by a different wrapper.");
        if (!committed.Remove(component.Identity))
            throw new InvalidOperationException("The canonical native component root could not be released.");
    }

    private static void ThrowCleanupFailures(List<Exception>? failures, string message)
    {
        if (failures is null || failures.Count == 0)
            return;
        if (failures.Count == 1)
            System.Runtime.ExceptionServices.ExceptionDispatchInfo.Capture(failures[0]).Throw();
        throw new AggregateException(message, failures);
    }
}
