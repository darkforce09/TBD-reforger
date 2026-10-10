# Eden_Editor:_Layer

Source: https://community.bohemia.net/wiki/Eden_Editor:_Layer

---

A layer is like a folder that can contain multiple entities of type object, group, trigger, waypoint, system or marker.
They can also contain sub-layers. Layers can be useful in various ways such as:

- Tidying up the editor view while editing by hiding the layers and their entities

- Grouping certain entities together thematically

- Preventing a group of entities from being accidentally edited by disabling transformation

- Dynamically creating the entities within a layer during the scenario

### Creating a layer

There multiple ways to create layers:

- Select entities in the 3D or map view or the entity list (left panel) and press the New Layer button in the entity list

- Use the add3DENLayer scripting command

ⓘPressing the New Layer button without having entities selected will create an empty layer

### Deleting a layer

Layers can be deleted by either selecting them in the entity list and pressing the Del key or using the Delete button in the entity list.

⚠Deleting a layer will also delete all sub-layers and entities within it!

### Dynamically create layer entities

One advantage of layers is that they can be used to hide or show all entities within the layer dynamically via scripting during a scenario.

- Create a layer, name it NewAO and add some objects and markers to it.

- Select all objects in that layer and uncheck Show Model and Enable Simulation in the attributes. In case of markers, set the Alpha value to 0

- Preview the scenario

- All objects and markers will now be invisible

Open the Arma 3: Debug Console and execute the following code. If you use want to activate the layer via a trigger, make sure to check the Server only checkboxgetMissionLayerEntities "NewAO" params ["_objects", "_markers"]; // The command returns an array with two arrays. First array contains all objects, second array contains all makers
_markers apply { _x setMarkerAlpha 1; }; // Show all markers
_objects apply { _x enableSimulationGlobal true; _x hideObjectGlobal false; }; // Show all entities and enable their simulation

### Attributes

Info

Development

Name

Category

Description

Property

Type

| Name

| Init

| Name of the layer set in Eden Editor

| Name

| String

| Enable Transformation

| Init

| If set to false (unchecked), the layer cannot be modified inside Eden Editor.

| Transformation

| Boolean

| Enable Visibility

| Init

| If set to false (unchecked), the layer will not be shown inside Eden Editor. This will not affect visibility during the scenario!

| Visibility

| Boolean

Retrieved from "https://community.bistudio.com/wiki?title=Eden_Editor:_Layer&oldid=352005"
					Category: - Eden Editor: Asset Types