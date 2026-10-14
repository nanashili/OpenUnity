using OpenUnity.Interop;

namespace UnityEngine;

public partial class Transform
{
    public Transform? parent
    {
        get
        {
            long found = Context.Invoke(runtime => OpenUnityNative.ou_transform_parent(runtime, Identity));
            CheckNative(found, "parent query");
            return found == 0 ? null : GameObject.WrapTransform(Context, found);
        }
        set => SetParent(value, true);
    }

    public Transform root
    {
        get
        {
            Transform current = this;
            while (current.parent is Transform ancestor)
                current = ancestor;
            return current;
        }
    }

    public bool IsChildOf(Transform parent)
    {
        ArgumentNullException.ThrowIfNull(parent);

        Transform? current = this;
        while (current is not null)
        {
            if (ReferenceEquals(current, parent))
                return true;
            current = current.parent;
        }

        return false;
    }

    public int childCount => Context.Invoke(runtime =>
    {
        int count = OpenUnityNative.ou_transform_child_count(runtime, Identity);
        CheckNative(count, "child count query");
        return count;
    });

    public int GetSiblingIndex() => Context.Invoke(runtime =>
    {
        int index = OpenUnityNative.ou_transform_sibling_index(runtime, Identity);
        CheckNative(index, "sibling index query");
        return index;
    });

    public void SetSiblingIndex(int index)
    {
        ChangeSiblingOrder(
            runtime => OpenUnityNative.ou_transform_set_sibling_index(runtime, Identity, index),
            "sibling index assignment",
            index);
    }

    public void SetAsFirstSibling() => SetSiblingIndex(0);

    public void DetachChildren()
    {
        Context.CheckLifecycleThreadIfInitialized();
        Context.RequireTransformDetach();
        int status = Context.Invoke(runtime =>
            OpenUnityNative.ou_transform_detach_children(runtime, Identity));
        CheckNative(status, "child detachment");
        Context.PumpLifecycleIfInitialized();
    }

    public void SetAsLastSibling() => ChangeSiblingOrder(
        runtime => OpenUnityNative.ou_transform_set_as_last_sibling(runtime, Identity),
        "last sibling assignment");

    public Transform GetChild(int index)
    {
        long found = Context.Invoke(runtime => OpenUnityNative.ou_transform_child(runtime, Identity, index));
        if (found == -8)
            throw new UnityException("Transform child out of bounds");
        CheckNative(found, "child query");
        return GameObject.WrapTransform(Context, found);
    }

    public Transform? Find(string path)
    {
        ArgumentNullException.ThrowIfNull(path);
        if (path.Length == 0)
            return null;

        Transform current = this;
        foreach (string segment in path.Split('/'))
        {
            Transform? child = FindDirectChild(current, segment);
            if (child is null)
                return null;
            current = child;
        }

        return current;
    }

    public void SetParent(Transform? parent, bool worldPositionStays)
    {
        Context.CheckLifecycleThreadIfInitialized();
        Context.Invoke(runtime =>
        {
            long parentId = ReferenceEquals(parent, null) ? 0 : parent.Identity;
            int status = OpenUnityNative.ou_transform_set_parent(runtime, Identity, parentId, worldPositionStays ? 1 : 0);
            if (status != 0)
            {
                CheckNative(status, "parent assignment");
                throw RuntimeContext.Failure("parent assignment", status);
            }
            return status;
        });
        Context.PumpLifecycleIfInitialized();
    }

    public void SetParent(Transform? parent) => SetParent(parent, true);

    private static Transform? FindDirectChild(Transform parent, string name)
    {
        int count = parent.childCount;
        for (int index = 0; index < count; index++)
        {
            Transform child = parent.GetChild(index);
            if (string.Equals(child.name, name, StringComparison.Ordinal))
                return child;
        }

        return null;
    }

    private void ChangeSiblingOrder(Func<long, int> nativeOperation, string operation, int? index = null)
    {
        Context.CheckLifecycleThreadIfInitialized();
        int status = Context.Invoke(runtime =>
        {
            Context.RequireTransformHierarchy();
            try
            {
                return nativeOperation(runtime);
            }
            catch (EntryPointNotFoundException error)
            {
                throw new NotSupportedException("OpenUnity Transform hierarchy ABI 1.0 is unavailable.", error);
            }
        });

        ThrowSiblingOrderFailure(status, operation, index);
        Context.PumpLifecycleIfInitialized();
    }

    private static void ThrowSiblingOrderFailure(int status, string operation, int? index)
    {
        if (status == -20 && index is int invalidIndex)
            throw new ArgumentOutOfRangeException(
                nameof(index), invalidIndex, "Sibling index must identify an existing child or root position.");
        if (status == -8)
            throw new InvalidOperationException(
                "OpenUnity cannot reorder a malformed or mixed-ownership Transform group.");
        if (status == -2)
            throw new MissingReferenceException("The OpenUnity Transform has been destroyed.");
        if (status == -11)
            throw new NotSupportedException(
                "OpenUnity cannot reorder an unowned root without an ordered scene group.");
        if (status == -17)
            throw new InvalidOperationException(
                "OpenUnity cannot reorder this Transform while scene staging is active or from a non-owner thread.");
        if (status != 0)
        {
            CheckNative(status, operation);
            throw RuntimeContext.Failure(operation, status);
        }
    }
}
