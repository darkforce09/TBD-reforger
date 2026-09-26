/**
 * @file TBD_MissionVariantSources.c
 * @brief Reads the variant inputs the typed parse cannot see: the server override file, per-row
 * default flags, slot and vehicle gates, and excluded vehicles' crew seats.
 *
 * Role: every raw-JSON and profile-file read the variant filter needs.  Position: called by
 * `TBD_MissionVariantFilter`; reads `$profile:TBD_VariantConfig.json` and the raw mission text.
 * State: none.  Invariants: the raw walkers track string, escape and nesting state, match a key only
 * at the depth they name, and return index-aligned rows (one entry per array row, in order); an
 * override file that exists but cannot be read is an ERROR and the caller falls back to document
 * defaults.
 */

//! The server-side variant selection in `$profile:TBD_VariantConfig.json`. A separate file from
//! `TBD_BackendConfig.json`, because `TBD_BackendConfig.Save` rewrites that file with only its own
//! fields.
class TBD_VariantConfigWire
{
	ref array<string> activeVariants; //!< JSON `activeVariants`: explicit selection that replaces document defaults; an authored empty array selects none.
}

//! Static readers of the variant inputs outside the typed mission parse.
class TBD_MissionVariantSources
{
	static const string OVERRIDE_CONFIG_PATH = "$profile:TBD_VariantConfig.json"; //!< Server override file; absent means no override.

	//! Read the server override file, following the pattern of `TBD_BackendConfig.Load`.
	//! `JsonLoadContext` allocates `activeVariants` even when the key is absent, so key presence is
	//! read off the raw file bytes.
	//! @param keyPresent set true when the file authors a top-level `activeVariants` key
	//! @return the parsed override, or null when the file is missing (silently) or cannot be
	//! opened, read or parsed (one ERROR each)
	static ref TBD_VariantConfigWire ReadVariantOverride(out bool keyPresent)
	{
		keyPresent = false;

		if (!FileIO.FileExists(OVERRIDE_CONFIG_PATH))
			return null;

		FileHandle handle = FileIO.OpenFile(OVERRIDE_CONFIG_PATH, FileMode.READ);
		if (!handle)
		{
			Print("[TBD][Variants] could not OPEN " + OVERRIDE_CONFIG_PATH + " -- override ignored, using document defaults", LogLevel.ERROR);
			return null;
		}

		string raw;
		handle.Read(raw, TBD_MissionLoader.MISSION_FILE_MAX_BYTES);
		handle.Close();

		JsonLoadContext ctx = new JsonLoadContext();
		if (!ctx.LoadFromString(raw))
		{
			Print("[TBD][Variants] could not READ " + OVERRIDE_CONFIG_PATH + " -- override ignored, using document defaults", LogLevel.ERROR);
			return null;
		}

		TBD_VariantConfigWire cfg = new TBD_VariantConfigWire();
		if (!ctx.ReadValue("", cfg))
		{
			Print("[TBD][Variants] could not PARSE " + OVERRIDE_CONFIG_PATH + " -- override ignored, using document defaults", LogLevel.ERROR);
			return null;
		}

		keyPresent = OverrideActiveVariantsKeyPresent(raw);
		return cfg;
	}

	//! Whether the override file authors a top-level `activeVariants` key (a depth-1 key directly
	//! before `:`).
	//! @param raw the override file text
	//! @return true when the key is present, whatever its value
	static bool OverrideActiveVariantsKeyPresent(string raw)
	{
		int length = raw.Length();
		int depth = 0;
		bool inString = false;
		bool escaped = false;
		bool capture = false;
		string token = "";
		string lastToken = "";
		int lastTokenDepth = -1;

		for (int i = 0; i < length; i++)
		{
			string c = raw.Substring(i, 1);

			if (inString)
			{
				if (escaped)
				{
					escaped = false;
					continue;
				}
				if (c == "\\")
				{
					escaped = true;
					continue;
				}
				if (c == "\"")
				{
					inString = false;
					lastToken = token;
					lastTokenDepth = depth;
					continue;
				}
				if (capture)
					token = token + c;
				continue;
			}

			if (c == " " || c == "\t" || c == "\n" || c == "\r")
				continue;

			if (c == "\"")
			{
				inString = true;
				token = "";
				capture = (depth == 1);
				continue;
			}

			if (c == ":")
			{
				if (lastTokenDepth == 1 && lastToken == "activeVariants")
					return true;
				continue;
			}

			if (c == "{" || c == "[")
			{
				depth = depth + 1;
				continue;
			}

			if (c == "}" || c == "]")
			{
				depth = depth - 1;
				continue;
			}
		}

		return false;
	}

	//! Second typed pass over the held mission text for the slot and vehicle variant gates.
	//! @return the gate skeleton, or null when the text is missing or does not parse
	static ref TBD_VariantGateSkeletonStruct ParseVariantGateSkeleton()
	{
		TBD_EMissionJsonPassOutcome outcome;
		JsonLoadContext ctx = TBD_MissionJsonPass.LoadRoot(outcome);
		if (!ctx)
			return null;

		TBD_VariantGateSkeletonStruct skeleton = new TBD_VariantGateSkeletonStruct();
		if (!ctx.ReadValue("", skeleton))
			return null;

		return skeleton;
	}

	//! The crew `slotId` values of every `vehicles[]` row, read off the raw JSON so the typed
	//! crew-plan member is never named here. `slotId` occurs only in that nested crew plan.
	//! @param raw the mission text
	//! @return one array of crew slot ids per vehicle row, index-aligned with `vehicles[]`
	static ref array<ref array<string>> ExtractVehicleCrewSlotIds(string raw)
	{
		array<ref array<string>> rows = new array<ref array<string>>();

		int length = raw.Length();
		int depth = 0;
		bool inString = false;
		bool escaped = false;
		bool capture = false;
		string token = "";
		string lastToken = "";
		int lastTokenDepth = -1;
		int vehiclesDepth = -1;
		int rowIndex = -1;
		bool expectVehiclesArray = false;
		bool expectSlotIdString = false;

		for (int i = 0; i < length; i++)
		{
			string c = raw.Substring(i, 1);

			if (inString)
			{
				if (escaped)
				{
					escaped = false;
					continue;
				}
				if (c == "\\")
				{
					escaped = true;
					continue;
				}
				if (c == "\"")
				{
					inString = false;
					if (expectSlotIdString && rowIndex >= 0 && rowIndex < rows.Count())
					{
						rows[rowIndex].Insert(token);
						expectSlotIdString = false;
					}
					lastToken = token;
					lastTokenDepth = depth;
					continue;
				}
				if (capture || expectSlotIdString)
					token = token + c;
				continue;
			}

			if (c == " " || c == "\t" || c == "\n" || c == "\r")
				continue;

			if (c == "\"")
			{
				inString = true;
				token = "";
				capture = (depth == 1) || (vehiclesDepth != -1 && depth >= vehiclesDepth);
				continue;
			}

			if (c == ":")
			{
				if (lastTokenDepth == 1 && lastToken == "vehicles" && vehiclesDepth == -1)
					expectVehiclesArray = true;
				else if (vehiclesDepth != -1 && rowIndex >= 0 && lastToken == "slotId")
					expectSlotIdString = true;
				continue;
			}

			if (c == "{" || c == "[")
			{
				if (expectVehiclesArray)
				{
					expectVehiclesArray = false;
					if (c == "[")
						vehiclesDepth = depth + 1;
				}
				else if (vehiclesDepth != -1 && c == "{" && depth == vehiclesDepth)
				{
					rowIndex = rowIndex + 1;
					array<string> crewIds = new array<string>();
					rows.Insert(crewIds);
				}
				expectSlotIdString = false;
				depth = depth + 1;
				continue;
			}

			if (c == "}" || c == "]")
			{
				depth = depth - 1;
				if (vehiclesDepth != -1 && depth < vehiclesDepth)
					break;
				continue;
			}

			expectVehiclesArray = false;
			expectSlotIdString = false;
		}

		return rows;
	}

	//! The per-row `default` flags of the top-level `variants[]` array, read off the raw JSON
	//! because `default` is an Enforce keyword the typed parser cannot map.
	//! @param raw the mission text
	//! @param variantsKeyPresent set true when a top-level `variants` array is found, even empty
	//! @return one flag per `variants[]` row, index-aligned; true only for `default: true`
	static ref array<bool> ExtractVariantDefaultFlags(string raw, out bool variantsKeyPresent)
	{
		array<bool> flags = new array<bool>();
		variantsKeyPresent = false;

		int length = raw.Length();
		int depth = 0;
		bool inString = false;
		bool escaped = false;
		bool capture = false;
		string token = "";
		string lastToken = "";
		int lastTokenDepth = -1;
		int variantsDepth = -1;
		int rowIndex = -1;
		bool expectVariantsArray = false;
		bool expectDefaultLiteral = false;

		for (int i = 0; i < length; i++)
		{
			string c = raw.Substring(i, 1);

			if (inString)
			{
				if (escaped)
				{
					escaped = false;
					continue;
				}
				if (c == "\\")
				{
					escaped = true;
					continue;
				}
				if (c == "\"")
				{
					inString = false;
					lastToken = token;
					lastTokenDepth = depth;
					continue;
				}
				if (capture)
					token = token + c;
				continue;
			}

			if (c == " " || c == "\t" || c == "\n" || c == "\r")
				continue;

			if (c == "\"")
			{
				inString = true;
				token = "";
				capture = (depth == 1) || (variantsDepth != -1 && depth == variantsDepth + 1);
				expectVariantsArray = false;
				expectDefaultLiteral = false;
				continue;
			}

			if (c == ":")
			{
				if (lastTokenDepth == 1 && lastToken == "variants" && variantsDepth == -1)
					expectVariantsArray = true;
				else if (variantsDepth != -1 && rowIndex >= 0 && lastTokenDepth == variantsDepth + 1 && lastToken == "default")
					expectDefaultLiteral = true;
				continue;
			}

			if (c == "{" || c == "[")
			{
				if (expectVariantsArray)
				{
					expectVariantsArray = false;
					if (c == "[")
					{
						variantsDepth = depth + 1;
						variantsKeyPresent = true;
					}
				}
				else if (variantsDepth != -1 && c == "{" && depth == variantsDepth)
				{
					rowIndex = rowIndex + 1;
					flags.Insert(false);
				}
				else
				{
					expectDefaultLiteral = false;
				}
				depth = depth + 1;
				continue;
			}

			if (c == "}" || c == "]")
			{
				depth = depth - 1;
				if (variantsDepth != -1 && depth < variantsDepth)
					break;
				continue;
			}

			expectVariantsArray = false;

			if (expectDefaultLiteral && c != ",")
			{
				if (c == "t" && rowIndex >= 0 && rowIndex < flags.Count())
					flags[rowIndex] = true;
				expectDefaultLiteral = false;
			}
		}

		return flags;
	}
}
