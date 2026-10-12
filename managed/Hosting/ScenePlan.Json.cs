using System.Text.Json;

namespace OpenUnity.Hosting;

internal sealed partial class ScenePlan
{
    private static JsonElement Shape(JsonElement element, params string[] keys)
    {
        if (element.ValueKind != JsonValueKind.Object || element.EnumerateObject().Count() != keys.Length ||
            keys.Any(key => !element.TryGetProperty(key, out _)))
            throw Invalid("Unknown or missing scene plan key.");
        return element;
    }

    private static JsonElement Array(JsonElement element) => element.ValueKind == JsonValueKind.Array
        ? element : throw Invalid("Scene plan array required.");
    private static int Integer(JsonElement element) => element.ValueKind == JsonValueKind.Number &&
        element.TryGetInt32(out int value)
        ? value : throw Invalid("Scene plan Int32 required.");
    private static long Reference(JsonElement element) => element.ValueKind == JsonValueKind.Number &&
        element.TryGetInt64(out long value) && value >= 0
        ? value : throw Invalid("Scene plan nonnegative Int64 reference required.");
    private static long Id(JsonElement element)
    {
        long value = Reference(element);
        return value > 0 ? value : throw Invalid("Scene plan identity must be positive.");
    }
    private static bool Boolean(JsonElement element) => element.ValueKind is JsonValueKind.True or JsonValueKind.False
        ? element.GetBoolean() : throw Invalid("Scene plan Boolean required.");
    private static float Float(JsonElement element) => element.ValueKind == JsonValueKind.Number &&
        element.TryGetSingle(out float value) && float.IsFinite(value)
        ? value : throw Invalid("Scene plan finite Single required.");
    private static float[] Floats(JsonElement element, int count)
    {
        Array(element);
        if (element.GetArrayLength() != count)
            throw Invalid("Scene plan vector length mismatch.");
        return element.EnumerateArray().Select(Float).ToArray();
    }
    private static long[] ReadIds(JsonElement element)
    {
        Array(element);
        if (element.GetArrayLength() > MaximumObjects)
            throw Invalid("Scene identity array limit exceeded.");
        return element.EnumerateArray().Select(Id).ToArray();
    }
    private static string Text(JsonElement element) => element.ValueKind == JsonValueKind.String
        ? element.GetString()! : throw Invalid("Scene plan string required.");

    private static void ValidateStringsAndDuplicates(JsonElement element)
    {
        if (element.ValueKind == JsonValueKind.Object)
        {
            HashSet<string> keys = new(StringComparer.Ordinal);
            foreach (JsonProperty property in element.EnumerateObject())
            {
                if (Utf8.GetByteCount(property.Name) > 1_048_576 || !keys.Add(property.Name))
                    throw Invalid("Oversized or duplicate scene plan key.");
                ValidateStringsAndDuplicates(property.Value);
            }
        }
        else if (element.ValueKind == JsonValueKind.Array)
            foreach (JsonElement item in element.EnumerateArray())
                ValidateStringsAndDuplicates(item);
        else if (element.ValueKind == JsonValueKind.String && Utf8.GetByteCount(Text(element)) > 1_048_576)
            throw Invalid("Oversized scene plan string.");
    }

    private static ArgumentException Invalid(string message) => new(message);
}
