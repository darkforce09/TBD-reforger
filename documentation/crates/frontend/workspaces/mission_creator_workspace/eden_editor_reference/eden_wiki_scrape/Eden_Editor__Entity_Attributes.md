# Eden_Editor:_Entity_Attributes

Source: https://community.bohemia.net/wiki/Eden_Editor:_Entity_Attributes

---

An entity is defined by its attributes. Each entity type has a different set of attributes. Object attributes, for example, can be type, position or health, while a marker is configured by size or color values.

⚠'Disable Simulation' and 'Simple Object' attributes do not work well with custom animation attributes. Consider using object 'init' as workaround to achieve desired effect 

### Editing Attributes

Entity attributes can be tweaked in the Edit Attributes window. To open it, double-click on the entity.

Alternatively, you can click the right mouse button on an entity and pick Attributes option from the context menu.

#### Editing multiple entities

When multiple entities are selected, attributes will be modified for all of them. This is useful for batch changes, e.g., making multiple characters playable in multiplayer.

### Attribute Categories

Attributes in the Edit Attributes window are sorted into categories, each of which can be expanded and collapsed by clicking on their titles. Their state is remembered and restored after returning.

The description for each attribute is available in a tooltip when hovering over the attribute's title.
The modified values are only saved  after pressing OK; clicking on Cancel will discard all changes.

#### Editing multiple entities

When multiple entities are being edited, an attribute can only be edited if its value is shared by all of them. When it is not, the attribute is disabled and the original values will not be overwritten.
If you wish to edit the attribute, enable it using the checkbox on its right. The modified value will be applied to all the selected entities.

### Entity Specific Attributes

Some entities, in particular modules and other systems, may have specific attributes unique only to them. They are always shown as the last category called Entity Specific - [Entity Name].

Attributes are a cornerstone of the editing process. Also, be sure to check the Eden Editor: Scenario Attributes tutorial which will explain how to configure the whole scenario, not only the single entities.

Retrieved from "https://community.bistudio.com/wiki?title=Eden_Editor:_Entity_Attributes&oldid=364518"
					Category: - Eden Editor: Editing