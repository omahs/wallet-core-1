use super::*;
use crate::manifest::{PropertyInfo, TypeVariant};
use heck::ToLowerCamelCase;

/// This function checks each property and determines whether there's an
/// association with the passed on object (struct or enum), based on common name
/// prefix, and maps the data into a Swift structure.
///
/// This function returns a tuple of associated Swift properties and skipped
/// respectively non-associated properties.
pub(super) fn process_object_properties(
    object: &ObjectVariant,
    properties: Vec<PropertyInfo>,
) -> Result<(Vec<SwiftProperty>, Vec<PropertyInfo>)> {
    let mut swift_props = vec![];
    let mut skipped_props = vec![];

    for prop in properties {
        if !prop.name.starts_with(object.name()) {
            // Property is not assciated with the object.
            skipped_props.push(prop);
            continue;
        }

        let mut ops = vec![];

        // Initalize the 'self' type, which is then passed on to the underlying
        // C FFI function.
        ops.push(match object {
            // E.g. `let obj = self.rawValue`
            ObjectVariant::Struct(_) => SwiftOperation::Call {
                var_name: "obj".to_string(),
                call: "self.rawValue".to_string(),
                defer: None,
            },
            // E.g. `let obj = TWSomeEnum(rawValue: self.rawValue")`
            ObjectVariant::Enum(name) => SwiftOperation::Call {
                var_name: "obj".to_string(),
                call: format!("{}(rawValue: self.rawValue)", name),
                defer: None,
            },
        });

        // Call the underlying C FFI function, passing on the `obj` instance.
        //
        // E.g: `let result = TWSomeFunc(obj)`.
        if prop.return_type.is_nullable {
            ops.push(SwiftOperation::GuardedCall {
                var_name: "result".to_string(),
                call: format!("{}(obj)", prop.name),
            });
        } else {
            ops.push(SwiftOperation::Call {
                var_name: "result".to_string(),
                call: format!("{}(obj)", prop.name),
                defer: None,
            });
        }

        // The `result` must be handled and returned explicitly.
        ops.push(match &prop.return_type.variant {
            // E.g.`return TWStringNSString(result)`
            TypeVariant::String => SwiftOperation::Return {
                call: "TWStringNSString(result)".to_string(),
            },
            TypeVariant::Data => SwiftOperation::Return {
                call: "TWDataNSData(result)".to_string(),
            },
            // E.g. `return SomeEnum(rawValue: result.rawValue)`
            TypeVariant::Enum(_) => SwiftOperation::Return {
                call: format!(
                    "{}(rawValue: result.rawValue)!",
                    SwiftType::from(prop.return_type.variant.clone())
                ),
            },
            // E.g. `return SomeStruct(rawValue: result)`
            TypeVariant::Struct(_) => SwiftOperation::Return {
                call: format!(
                    "{}(rawValue: result)",
                    SwiftType::from(prop.return_type.variant.clone())
                ),
            },
            _ => SwiftOperation::Return {
                call: "result".to_string(),
            },
        });

        // Prettify name, remove object name prefix from this property.
        let pretty_name = prop
            .name
            .strip_prefix(object.name())
            // Panicing implies bug, checked at the start of the loop.
            .unwrap()
            .to_lower_camel_case();

        // Convert return type.
        let return_type = SwiftReturn {
            param_type: SwiftType::from(prop.return_type.variant),
            is_nullable: prop.return_type.is_nullable,
        };

        swift_props.push(SwiftProperty {
            name: pretty_name,
            is_public: prop.is_public,
            operations: ops,
            return_type,
            comments: vec![],
        });
    }

    Ok((swift_props, skipped_props))
}
