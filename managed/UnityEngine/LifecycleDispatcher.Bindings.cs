namespace UnityEngine;

internal sealed partial class LifecycleDispatcher
{
    internal void ReleaseSceneBindings(IEnumerable<long> scripts, IEnumerable<GameObject> sceneOwners)
    {
        CheckThread();
        HashSet<GameObject> releasedOwners = new(sceneOwners, ReferenceEqualityComparer.Instance);
        HashSet<GameObject> failedOwners = new(ReferenceEqualityComparer.Instance);
        List<Exception> failures = new();

        TryReleaseSceneNativeRoots(releasedOwners, failedOwners, failures);
        ValidateSceneOwnerRoots(releasedOwners, failedOwners, failures);
        RemoveReleasedSceneScripts(scripts, releasedOwners, failedOwners);
        RemoveReleasedSceneAttachments(releasedOwners, failedOwners);
        RemoveReleasedSceneOwners(releasedOwners, failedOwners);

        ThrowCleanupFailures(failures, "Scene attachment cleanup failed.");
    }

    internal void RetainOwner(GameObject owner)
    {
        CheckThread();
        owners[owner.Identity] = owner;
    }

    internal GameObject? FindOwner(long identity)
    {
        CheckThread();
        return owners.GetValueOrDefault(identity);
    }

    private void TryReleaseSceneNativeRoots(
        HashSet<GameObject> releasedOwners,
        HashSet<GameObject> failedOwners,
        List<Exception> failures)
    {
        foreach (GameObject owner in releasedOwners)
        {
            try
            {
                nativeComponents.ReleaseRetiredOwner(owner);
            }
            catch (Exception failure)
            {
                failedOwners.Add(owner);
                failures.Add(failure);
            }
        }
    }

    private void ValidateSceneOwnerRoots(
        HashSet<GameObject> releasedOwners,
        HashSet<GameObject> failedOwners,
        List<Exception> failures)
    {
        foreach (GameObject owner in releasedOwners)
        {
            if (!owners.TryGetValue(owner.Identity, out GameObject? existing) ||
                ReferenceEquals(existing, owner))
                continue;

            failedOwners.Add(owner);
            failures.Add(new InvalidOperationException(
                "OpenUnity scene owner identity is rooted by a different managed object."));
        }
    }

    private void RemoveReleasedSceneScripts(
        IEnumerable<long> scripts,
        HashSet<GameObject> releasedOwners,
        HashSet<GameObject> failedOwners)
    {
        foreach (long identity in scripts)
        {
            if (!attachments.TryGetValue(identity, out MonoBehaviour? script))
                continue;
            if (CanReleaseSceneOwner(script.AttachedOwner, releasedOwners, failedOwners))
                attachments.Remove(identity);
        }
    }

    private void RemoveReleasedSceneAttachments(
        HashSet<GameObject> releasedOwners,
        HashSet<GameObject> failedOwners)
    {
        foreach (long identity in attachments
                     .Where(item => CanReleaseSceneOwner(item.Value.AttachedOwner, releasedOwners, failedOwners))
                     .Select(item => item.Key)
                     .ToArray())
            attachments.Remove(identity);
    }

    private void RemoveReleasedSceneOwners(
        HashSet<GameObject> releasedOwners,
        HashSet<GameObject> failedOwners)
    {
        foreach (GameObject owner in releasedOwners)
        {
            if (!CanReleaseSceneOwner(owner, releasedOwners, failedOwners))
                continue;

            if (owners.TryGetValue(owner.Identity, out GameObject? existing) &&
                ReferenceEquals(existing, owner))
                owners.Remove(owner.Identity);
            GameObject.ReleaseSceneWrapper(owner);
        }
    }

    private static bool CanReleaseSceneOwner(
        GameObject owner,
        HashSet<GameObject> releasedOwners,
        HashSet<GameObject> failedOwners) =>
        releasedOwners.Contains(owner) && !failedOwners.Contains(owner);
}
