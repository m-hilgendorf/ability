# Ability # 

Ability (short for __ABI__ compatib**ility**) provides 
macros for defining traits with compatible ABI's across 
compiler versions. 

In the short term this allows for defining interfaces for application 
extensions distributed as shared libraries. Longer term, supporting more
complex data types (and maybe generics?) should be supported. 

Contributions are welcome 

## Usage ## 

```toml
[dependencies]
ability = { git = "https://github.com/m-hilgendorf/ability.git"}
```

To add to a trait in your library, just add the `#[interface]` attribute 
to your trait definition. 

```rust 
use ability::interface;

#[interface]
pub trait MyTrait {
    fn foo(&self);
}
```

To implement interfaces for a binary 

## Limitations

 For the moment, the passing of POD/C types is supported (pointers, integers, float, 
 structs marked `#[repr(C)]`, etc).
 
 If you need to pass more complicated data, serialize it and pass as a pointer + length. 

## Roadmap ## 

- [ ] Documentation/examples
- [ ] Error messaging/failures for invalid traits 
- [ ] Clear description of limitations 
- [ ] Support for generics ? 
- [ ] Support for serializable types ?
- [ ] C header code generation
- [ ] Simpler, safer API for binaries that load the traits