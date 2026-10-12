using System.Reflection;
using System.Runtime.Loader;

namespace UnityEngine;

internal static class NativeComponentBindingResolution
{
    private const string PhysicsModuleName = "UnityEngine.PhysicsModule";

    internal static NativeComponentBinding Resolve(
        Type requestedType,
        NativeComponentBindingOperation operation)
    {
        ArgumentNullException.ThrowIfNull(requestedType);
        if (!Enum.IsDefined(operation))
            throw new ArgumentOutOfRangeException(nameof(operation));

        ValidateRequestedType(requestedType);

        CustomAttributeData metadata = ReadBindingMetadata(requestedType);
        (Type adapterType, NativeComponentBindingCategory category) = ReadArguments(metadata);
        ValidateOperation(category, operation, requestedType);
        ValidateModuleIdentity(requestedType, adapterType);
        ValidateAdapterShape(adapterType, requestedType, category);

        INativeComponentQueryBinding adapter = MaterializeAdapter(adapterType);
        ValidateMaterializedAdapter(adapter, requestedType, category);
        return new NativeComponentBinding(requestedType, category, adapter);
    }

    private static void ValidateRequestedType(Type requestedType)
    {
        if (!requestedType.IsClass || requestedType.IsAbstract || requestedType.ContainsGenericParameters ||
            !IsPublicType(requestedType) || requestedType.BaseType != typeof(Component))
            throw InvalidBinding(requestedType, "the target must be a closed public Component class");
    }

    private static CustomAttributeData ReadBindingMetadata(Type requestedType)
    {
        List<CustomAttributeData> declarations = requestedType
            .GetCustomAttributesData()
            .Where(attribute => attribute.AttributeType == typeof(NativeComponentBindingAttribute))
            .ToList();

        if (declarations.Count != 1)
            throw InvalidBinding(requestedType, "exactly one direct binding declaration is required");

        return declarations[0];
    }

    private static (Type AdapterType, NativeComponentBindingCategory Category) ReadArguments(
        CustomAttributeData metadata)
    {
        if (metadata.ConstructorArguments.Count != 2 ||
            metadata.ConstructorArguments[0].ArgumentType != typeof(Type) ||
            metadata.ConstructorArguments[0].Value is not Type adapterType ||
            metadata.ConstructorArguments[1].ArgumentType != typeof(NativeComponentBindingCategory) ||
            !TryReadCategory(metadata.ConstructorArguments[1].Value, out NativeComponentBindingCategory category))
            throw new InvalidOperationException("Native component binding metadata has invalid arguments.");

        if (!Enum.IsDefined(category))
            throw new InvalidOperationException($"Native component binding category {category} is unsupported.");

        return (adapterType, category);
    }

    private static bool TryReadCategory(object? value, out NativeComponentBindingCategory category)
    {
        if (value is NativeComponentBindingCategory direct)
        {
            category = direct;
            return true;
        }

        if (value is int raw && Enum.IsDefined(typeof(NativeComponentBindingCategory), raw))
        {
            category = (NativeComponentBindingCategory)raw;
            return true;
        }

        category = default;
        return false;
    }

    private static void ValidateOperation(
        NativeComponentBindingCategory category,
        NativeComponentBindingOperation operation,
        Type requestedType)
    {
        if (operation == NativeComponentBindingOperation.Create &&
            category != NativeComponentBindingCategory.CreateAndQuery)
            throw new NotSupportedException(
                $"OpenUnity cannot create query-only native component {requestedType.FullName}.");
    }

    private static void ValidateModuleIdentity(Type requestedType, Type adapterType)
    {
        Assembly physicsAssembly = requestedType.Assembly;
        Assembly coreAssembly = typeof(Component).Assembly;
        AssemblyName physicsName = physicsAssembly.GetName();

        if (physicsName.Name != PhysicsModuleName || physicsName.Version != new Version(0, 0, 0, 0) ||
            !string.IsNullOrEmpty(physicsName.CultureName) || physicsName.GetPublicKeyToken() is { Length: > 0 } ||
            physicsAssembly.IsDynamic)
            throw InvalidBinding(requestedType, "the target is not from the admitted PhysicsModule assembly");

        if (!ReferenceEquals(requestedType.Module, adapterType.Module) ||
            !AssemblyIdentityMatches(physicsAssembly, adapterType.Assembly) ||
            AssemblyLoadContext.GetLoadContext(physicsAssembly) != AssemblyLoadContext.GetLoadContext(coreAssembly) ||
            !ReferencesAssembly(physicsAssembly, coreAssembly))
            throw InvalidBinding(requestedType, "target and adapter do not share the admitted Core load context");
    }

    private static void ValidateAdapterShape(
        Type adapterType,
        Type requestedType,
        NativeComponentBindingCategory category)
    {
        if (!adapterType.IsClass || adapterType.IsAbstract || adapterType.IsSealed == false ||
            adapterType.IsGenericType || adapterType.ContainsGenericParameters || adapterType.IsPublic ||
            !typeof(INativeComponentQueryBinding).IsAssignableFrom(adapterType))
            throw InvalidBinding(requestedType, "the adapter must be an internal sealed nongeneric binding type");

        ConstructorInfo? constructor = adapterType.GetConstructor(
            BindingFlags.Instance | BindingFlags.NonPublic,
            binder: null,
            Type.EmptyTypes,
            modifiers: null);
        if (constructor is null || !constructor.IsAssembly)
            throw InvalidBinding(requestedType, "the adapter must have an internal parameterless constructor");

        if (category == NativeComponentBindingCategory.CreateAndQuery &&
            !typeof(INativeComponentFactoryBinding).IsAssignableFrom(adapterType))
            throw InvalidBinding(requestedType, "a creating binding must implement the factory contract");
    }

    private static INativeComponentQueryBinding MaterializeAdapter(Type adapterType)
    {
        ConstructorInfo constructor = adapterType.GetConstructor(
            BindingFlags.Instance | BindingFlags.NonPublic,
            binder: null,
            Type.EmptyTypes,
            modifiers: null)!;

        try
        {
            return (INativeComponentQueryBinding)constructor.Invoke(null);
        }
        catch (TargetInvocationException error) when (error.InnerException is not null)
        {
            throw new InvalidOperationException(
                "The trusted native component adapter failed to initialize.",
                error.InnerException);
        }
    }

    private static void ValidateMaterializedAdapter(
        INativeComponentQueryBinding adapter,
        Type requestedType,
        NativeComponentBindingCategory category)
    {
        if (!ReferenceEquals(adapter.DeclaredType, requestedType) || adapter.Category != category)
            throw InvalidBinding(requestedType, "the materialized adapter does not match direct metadata");
    }

    private static bool ReferencesAssembly(Assembly assembly, Assembly actualCoreAssembly) =>
        assembly.GetReferencedAssemblies().Any(reference =>
            AssemblyName.ReferenceMatchesDefinition(actualCoreAssembly.GetName(), reference));

    private static bool AssemblyIdentityMatches(Assembly first, Assembly second) =>
        AssemblyName.ReferenceMatchesDefinition(first.GetName(), second.GetName()) &&
        AssemblyName.ReferenceMatchesDefinition(second.GetName(), first.GetName());

    private static bool IsPublicType(Type type) => type.IsPublic || type.IsNestedPublic;

    private static InvalidOperationException InvalidBinding(Type type, string reason) =>
        new($"Invalid native component binding for {type.FullName}: {reason}.");
}
