using OpenUnity.Interop;

namespace UnityEngine;

internal sealed partial class LifecycleDispatcher
{
    internal void ReleaseSceneBindings(IEnumerable<long> scripts, IEnumerable<GameObject> sceneOwners)
    {
        CheckThread();
        HashSet<GameObject> releasedOwners = new(sceneOwners, ReferenceEqualityComparer.Instance);
        foreach (long identity in scripts)
            attachments.Remove(identity);
        foreach (long identity in attachments.Where(item => releasedOwners.Contains(item.Value.AttachedOwner))
                     .Select(item => item.Key).ToArray())
            attachments.Remove(identity);
        foreach (GameObject owner in releasedOwners)
        {
            if (owners.TryGetValue(owner.Identity, out GameObject? existing) && ReferenceEquals(existing, owner))
                owners.Remove(owner.Identity);
            GameObject.ReleaseSceneWrapper(owner);
        }
    }

    internal Component[] Components(GameObject owner, Type type)
    {
        CheckThread();
        ArgumentNullException.ThrowIfNull(type);
        if (type != typeof(Transform) && type != typeof(Component) &&
            type != typeof(Behaviour) && type != typeof(MonoBehaviour) &&
            !(type.IsClass && typeof(MonoBehaviour).IsAssignableFrom(type)))
            throw new NotSupportedException($"OpenUnity component queries do not support {type.FullName}.");
        List<Component> found = new();
        context.Invoke(runtime =>
        {
            int count = OpenUnityNative.ou_gameobject_component_count(runtime, owner.Identity);
            Object.CheckNative(count, "component count query");
            for (int index = 0; index < count; index++)
            {
                long identity = OpenUnityNative.ou_gameobject_component_at(runtime, owner.Identity, index);
                Object.CheckNative(identity, "component identity query");
                Component component = index == 0 ? owner.transform : attachments.TryGetValue(identity, out MonoBehaviour? script)
                    ? script : throw new InvalidOperationException("OpenUnity returned a component without a canonical managed attachment.");
                if (type.IsAssignableFrom(component.GetType()))
                    found.Add(component);
            }
            return 0;
        });
        return found.ToArray();
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

    private void ReleaseDeadAttachments()
    {
        context.Invoke(runtime =>
        {
            foreach (long identity in attachments.Keys.ToArray())
                if (!IsAlive(runtime, identity))
                    attachments.Remove(identity);
            foreach (long identity in owners.Keys.ToArray())
                if (!IsAlive(runtime, identity))
                    owners.Remove(identity);
            return 0;
        });
    }

    private static bool IsAlive(long runtime, long identity) =>
        OpenUnityNative.ou_object_is_alive(runtime, identity) switch
        {
            0 => false,
            1 => true,
            int status => throw RuntimeContext.Failure("attachment liveness query", status)
        };
}
