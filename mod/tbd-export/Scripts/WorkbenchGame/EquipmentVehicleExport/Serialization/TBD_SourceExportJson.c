// Native JSON serialization preserves escaping and native scalar precision.
class TBD_SourceExportJson
{
	static string Unwrap(JsonSaveContext context)
	{
		string json = context.SaveToString();
		int start = json.IndexOf(":") + 1;
		int end = json.LastIndexOf("}");
		if (start < 1 || end < start || end != json.Length() - 1) return "null";
		return json.Substring(start, end - start);
	}

	static string Quote(string value)
	{
		// Small individual strings fit below the native serializer's output limit,
		// even when every input byte requires a six-byte JSON escape.
		if (value.Length() <= 512)
		{
			JsonSaveContext context = new JsonSaveContext(false);
			context.WriteValue("v", value);
			return Unwrap(context);
		}
		string encoded = "\"";
		string digits = "0123456789abcdef";
		for (int i = 0; i < value.Length(); i++)
		{
			string character = value.Substring(i, 1);
			int code = value.ToAscii(i);
			if (character == "\"" || character == "\\") encoded += "\\" + character;
			else if (code >= 0 && code < 32)
			{
				encoded += "\\u00" + digits.Substring(code / 16, 1);
				encoded += digits.Substring(code % 16, 1);
			}
			else encoded += character;
		}
		return encoded + "\"";
	}

	static string Nullable(string value)
	{
		if (value.IsEmpty()) return "null";
		return Quote(value);
	}

	static string Scalar(float value)
	{
		JsonSaveContext context = new JsonSaveContext(false);
		context.SetMaxDecimalPlaces(324);
		context.WriteValue("v", value);
		return Unwrap(context);
	}

	static string Scalars(array<float> values)
	{
		array<string> encoded = {};
		foreach (float value : values) encoded.Insert(Scalar(value));
		return "[" + Join(encoded) + "]";
	}

	static string Integers(array<int> values)
	{
		array<string> encoded = {};
		foreach (int value : values) encoded.Insert(value.ToString());
		return "[" + Join(encoded) + "]";
	}

	static string Booleans(array<bool> values)
	{
		array<string> encoded = {};
		foreach (bool value : values)
		{
			if (value) encoded.Insert("true");
			else encoded.Insert("false");
		}
		return "[" + Join(encoded) + "]";
	}

	static string Strings(array<string> values)
	{
		array<string> encoded = {};
		foreach (string value : values) encoded.Insert(Quote(value));
		return "[" + Join(encoded) + "]";
	}

	static string VectorValue(vector value)
	{
		array<float> values = { value[0], value[1], value[2] };
		return Scalars(values);
	}

	static string Join(array<string> values)
	{
		// Native joining allocates the final buffer once for large source snapshots.
		return string.Join(",", values, false);
	}

	static string PointerPart(string value)
	{
		value.Replace("~", "~0");
		value.Replace("/", "~1");
		return value;
	}

	static bool Write(string path, string content)
	{
		FileHandle file = FileIO.OpenFile(path, FileMode.WRITE);
		if (!file) return false;
		int written = file.Write(content);
		file.Close();
		return written == content.Length();
	}
}
