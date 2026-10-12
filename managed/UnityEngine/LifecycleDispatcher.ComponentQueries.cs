using OpenUnity.Interop;

namespace UnityEngine;

internal sealed partial class LifecycleDispatcher
{
    internal Component[] Components(GameObject owner, Type type)
    {
        CheckThread();
        ArgumentNullException.ThrowIfNull(owner);
        ArgumentNullException.ThrowIfNull(type);

        if (type == typeof(Transform))
            return [owner.transform];

        ValidateQueryType(type);
        RequirePhysicsCatalog();

        return context.Invoke(runtime => ReadCompleteCatalog(runtime, owner, type));
    }

    private void ValidateQueryType(Type type)
    {
        if (type == typeof(Component) || type == typeof(Behaviour) ||
            type == typeof(MonoBehaviour) ||
            (type.IsClass && typeof(MonoBehaviour).IsAssignableFrom(type)))
            return;

        NativeComponentBinding binding = ResolveNativeBinding(
            type,
            NativeComponentBindingOperation.Query);
        if (binding.Category is not NativeComponentBindingCategory.CreateAndQuery and
            not NativeComponentBindingCategory.QueryOnly)
            throw new NotSupportedException($"OpenUnity component queries do not support {type.FullName}.");
    }

    private static void RequirePhysicsCatalog()
    {
        uint version = OpenUnityNative.ou_physics_motion_abi_version();
        if (version != OpenUnityNative.PhysicsMotionAbiVersion)
            throw new NotSupportedException(
                $"OpenUnity complete component queries require Physics motion ABI 1.0; received 0x{version:X8}.");
    }

    private Component[] ReadCompleteCatalog(long runtime, GameObject owner, Type requested)
    {
        int count = OpenUnityNative.ou_physics_component_count(runtime, owner.Identity);
        Object.CheckNative(count, "complete component count query");

        List<Component> found = new();
        for (int index = 0; index < count; index++)
        {
            long identity = OpenUnityNative.ou_physics_component_at(runtime, owner.Identity, index);
            Object.CheckNative(identity, "complete component identity query");
            Component component = ResolveCatalogComponent(owner, identity, index);
            if (requested.IsAssignableFrom(component.GetType()))
                found.Add(component);
        }

        return found.ToArray();
    }

    private Component ResolveCatalogComponent(GameObject owner, long identity, int index)
    {
        if (index == 0)
        {
            if (identity != owner.transform.Identity)
                throw new InvalidOperationException(
                    "OpenUnity complete component catalog did not return Transform first.");
            return owner.transform;
        }

        if (attachments.TryGetValue(identity, out MonoBehaviour? script))
            return ValidateCatalogOwner(script, owner, identity);

        Component? native = nativeComponents.Find(identity);
        if (native is null)
            throw new InvalidOperationException(
                $"OpenUnity returned an unknown component identity {identity}.");
        return ValidateCatalogOwner(native, owner, identity);
    }

    private static Component ValidateCatalogOwner(
        Component component,
        GameObject expectedOwner,
        long identity)
    {
        if (component.Identity != identity ||
            !ReferenceEquals(component.AttachedOwner, expectedOwner) ||
            !ReferenceEquals(component.Context, expectedOwner.Context))
            throw new InvalidOperationException(
                "OpenUnity returned a component with inconsistent owner or context.");
        return component;
    }
}
