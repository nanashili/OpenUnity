using System.Text.Json;
using UnityEngine;

namespace OpenUnity.Hosting;

internal sealed partial class ScenePlan
{
    private static Dictionary<long, long> ReadTransforms(JsonElement transforms, Dictionary<long, Type> identities)
    {
        Dictionary<long, long> transformOwners = new();
        foreach (JsonElement transform in transforms.EnumerateArray())
        {
            Shape(transform, "file_id", "owner", "parent", "children", "local_position", "local_rotation", "local_scale");
            long id = Id(transform.GetProperty("file_id"));
            AddIdentity(identities, id, typeof(Transform));
            transformOwners.Add(id, Id(transform.GetProperty("owner")));
            _ = Reference(transform.GetProperty("parent"));
            ReadIds(transform.GetProperty("children"));
            _ = Floats(transform.GetProperty("local_position"), 3);
            float[] rotation = Floats(transform.GetProperty("local_rotation"), 4);
            if (rotation.All(value => value == 0))
                throw Invalid("Transform rotation cannot be zero.");
            _ = Floats(transform.GetProperty("local_scale"), 3);
        }
        return transformOwners;
    }

    private static List<Owner> ReadOwners(JsonElement gameObjects, Dictionary<long, Type> identities,
        Dictionary<long, long> transformOwners)
    {
        List<Owner> owners = new();
        foreach (JsonElement owner in gameObjects.EnumerateArray())
        {
            Shape(owner, "file_id", "name", "active", "components");
            long id = Id(owner.GetProperty("file_id"));
            AddIdentity(identities, id, typeof(GameObject));
            _ = Text(owner.GetProperty("name"));
            _ = Boolean(owner.GetProperty("active"));
            long[] components = ReadIds(owner.GetProperty("components"));
            if (components.Length == 0 || !transformOwners.TryGetValue(components[0], out long actualOwner) || actualOwner != id)
                throw Invalid("GameObject requires its mandatory Transform first.");
            owners.Add(new Owner(id, components[0]));
        }
        return owners;
    }

    private static void AddIdentity(Dictionary<long, Type> identities, long id, Type type)
    {
        if (!identities.TryAdd(id, type))
            throw Invalid("Duplicate scene local file ID.");
    }
}
