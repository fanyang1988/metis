# Marcos in ink!

在这篇文档中, 我们会深入探讨ink!中的宏的行为与作用, 以此作为认识metis宏行为的基础.

## Substract合约执行流程

首先我们看下合约编译出的wasm, 我们可以使用wasm工具显示可读的wasm字节码:

```wasm
(module
  (type $t0 (func (param i32 i32)))
  ...
  (type $t8 (func (param i32) (result i64)))
  (import "seal0" "seal_get_storage" (func $seal0.seal_get_storage (type $t6)))
  (import "seal0" "seal_set_storage" (func $seal0.seal_set_storage (type $t1)))
  ...
  (import "seal0" "seal_return" (func $seal0.seal_return (type $t1)))
  (import "env" "memory" (memory $env.memory 2 16))
  (func $f8 (type $t0) (param $p0 i32) (param $p1 i32)
  ...
  (func $deploy (type $t5) (result i32)
  ...
  (func $call (type $t5) (result i32)
  ...
  (global $g0 (mut i32) (i32.const 65536))
  (export "deploy" (func $deploy))
  (export "call" (func $call))
  (data $d0 (i32.const 65536) "Flipper::FlipFlipper::Flip::from\01\00\00\00\02\00\00\00\03\00\00\00\04\00\00\00\05\00\00\00\06\00\00\00\07\00\00\00\08")
)
```

即使不熟悉wasm, 也可以大致看出整个合约代码的结构, 值得关注的如下:

- type $t0 - $t1 : 定义了一系列函数类型
- import xxx : 导入的接口, 这些是链提供的API
- func xxx : 各种函数实现
- export xxx : 导出的函数, 可以被外部调用

链向合约执行上下文导入链的回调接口, 而链在执行合约时, 将会调用导出的`deploy`或者`call`接口, 在这个过程中, 这些接口的实现将会使用链的回调接口完成逻辑.
我们编译基于`ink!`的合约时, 使用的是标准的rust编译出wasmAssembly字节码, `cargo contract build` 命令只是附带生产metadata并加入一部分优化生成代码的参数.
因此`ink!`的主要目标, 就是帮助开发者将我们的代码通过各种库封装与宏展开生成上面的wasm字节码.

需要注意到, 在我们使用`ink!`时, 往往合约编写为以下形式的代码:

```rust
use ink_lang as ink;

#[ink::contract]
pub mod flipper {
    #[ink(storage)]
    pub struct Flipper {
        value: bool,
        value2: bool,
    }

    /// Event emitted when a token transfer occurs.
    #[ink(event)]
    pub struct Flip {
        #[ink(topic)]
        from: Option<AccountId>,
        value: bool,
    }

    #[ink(event)]
    pub struct EvtTest {
        #[ink(topic)]
        value: bool,
    }

    impl Flipper {
        /// Creates a new flipper smart contract initialized with the given value.
        #[ink(constructor)]
        pub fn new(init_value: bool) -> Self {
            Self { value: init_value, value2: init_value }
        }

        /// Creates a new flipper smart contract initialized to `false`.
        #[ink(constructor)]
        pub fn default() -> Self {
            Self::new(Default::default())
        }

        /// Flips the current value of the Flipper's bool.
        #[ink(message)]
        pub fn flip(&mut self) {
            let caller = Self::env().caller();
            self.value = !self.value;

            Self::env().emit_event(Flip {
                from: Some(caller),
                value: self.value,
            });
        }

        /// Returns the current value of the Flipper's bool.
        #[ink(message)]
        pub fn get(&self) -> bool {
            self.value
        }
    }
}
```

这是一个简化的合约代码, 下面我们主要基于这个合约代码作为例子, 这里并没有定义`deploy`或者`call`, 而是Flipper的`new`, `flip`和`get`几个message, 这是因为`ink!`通过宏会将这些message拼装为`deploy`和`call`实现, 与以太坊类似, 在`ink!`合约中, 我们对于输入有如下约定: 输入的前四个字节为message的函数`Selector`, 参见`ink!`中的定义:

```rust
/// A function selector.
///
/// # Note
///
/// This is equal to the first four bytes of the SHA-3 hash of a function's name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Selector {
    bytes: [u8; 4],
}
```

`call`函数中, 会基于不同的`Selector`来选择执行对应的逻辑, 在metadata中, 这些信息也可以找到:

```json
    {
        "messages": [
            {
                "args": [],
                "docs": [" Flips the current value of the Flipper's bool."],
                "mutates": true,
                "name": ["flip"],
                "payable": false,
                "returnType": null,
                "selector": "0x633aa551"
            },
        ]
    }
```

但从上述叙述中可以得知, `Selector`并不是一个强制规定, 事实上, 只要实现了`call`, 合约的开发者可以任意定义数据的格式与解释, 但是我们需要一个通行的标准来辅助最终用户来使用合约.

下面我们将会分析`ink!`的实现, 通过对`ink!`机制特别是宏展开的机制的了解, 我们可以更好地开发基于`ink!`的智能合约.

## ink!宏展开过程

每个初见`ink!`合约的开发者都会注意到, `ink!`中包含了大量的rust宏, 下面我们重点分析宏展开的结果与过程, 不熟悉rust宏的开发者可以参考:

- [Rust Latam: procedural macros workshop](https://github.com/dtolnay/proc-macro-workshop)

从上述例子中, 我们可以简单分析出`ink!`合约中的宏:

```rust
use ink_lang as ink;

#[ink::contract]
pub mod flipper {
    #[ink(storage)]
    pub struct Flipper { /* ... */ }

    #[ink(event)]
    pub struct Flip { /* ... */ }

    impl Flipper {
        #[ink(constructor)]
        pub fn new(init_value: bool) -> Self { /* ... */ }

        #[ink(message)]
        pub fn flip(&mut self) { /* ... */ }

        #[ink(message)]
        pub fn get(&self) -> bool { /* ... */ }
    }
}
```

在研究宏展开时, 最佳的辅助工具便是`cargo-expand`, 基于这个工具可以获得展开后的代码, 通过对其比较可以了解宏做了什么:

```bash
cargo install cargo-expand
cargo expand > ./expand-lib.rs
```

在`ink!`中, **当前**大部分宏展开的入口是`#[ink_lang::contract]`, 其余宏大多在其处理的代码块之下, 除此之外**目前**有如下几个宏:

- `#[ink_lang::contract]` 定义合约入口
- `#[ink_lang::trait_definition]` 定义一个面向合约的trait定义
- `#[ink_lang::test]` 链下测试支持
- `#[ink_lang::chain_extension]` 为特定的链定义其拓展, 以此来与其runtime层交互

这些宏定义在[macro](https://github.com/paritytech/ink/blob/master/crates/lang/macro/src/lib.rs)包中, 我们这里先看看`#[ink_lang::contract]`宏.

`#[ink_lang::contract]`宏的实际处理逻辑都在[codegen](https://github.com/paritytech/ink/blob/master/crates/lang/codegen/src/lib.rs)包中:

`codegen`中主要是定义总体的接口, 实现在`generator`中, 对于不同的类型专门做定义, 这里注意到, 对于所有的宏展开过程都是共享全部的上下文信息:

```rust
pub fn generate_or_err(attr: TokenStream2, input: TokenStream2) -> Result<TokenStream2> {
    let contract = Contract::new(attr, input)?;
    Ok(generate_code(&contract))
}
```

这里`Contract`如下:

```rust
/// Generates code for the entirety of the ink! contract.
#[derive(From)]
pub struct Contract<'a> {
    /// The contract to generate code for.
    contract: &'a ir::Contract,
}
```

`ir::Contract`包含了`ink!`生成代码所需的信息:

```rust
/// An ink! contract definition consisting of the ink! configuration and module.
///
/// This is the root of any ink! smart contract definition. It contains every
/// information accessible to the ink! smart contract macros. It is also used
/// as the root source for the ink! code generation.
///
/// # Example
///
/// ```no_compile
/// #[ink::contract(/* optional ink! configurations */)]
/// mod my_contract {
///     /* ink! and Rust definitions */
/// }
/// ```
pub struct Contract {
    /// The parsed Rust inline module.
    ///
    /// Contains all Rust module items after parsing. Note that while parsing
    /// the ink! module all ink! specific items are moved out of this AST based
    /// representation.
    item: ir::ItemMod,
    /// The specified ink! configuration.
    config: ir::Config,
}
```

后续我们在过程中使用到这些信息时, 在做深入分析.

在[generator/contract.rs](https://github.com/paritytech/ink/blob/master/crates/lang/codegen/src/generator/contract.rs)中,
有`#[ink_lang::contract]`宏生成代码的实现:

```rust
    /// Generates ink! contract code.
    fn generate_code(&self) -> TokenStream2 {
        // 一些基本信息
        let module = self.contract.module();
        let ident = module.ident(); // mod的名称
        let attrs = module.attrs(); // mod的其他宏参数
        let vis = module.vis(); // mod的可见性

        // 对各个宏所修饰的块进行分别的生成
        let env = self.generate_code_using::<generator::Env>();
        let storage = self.generate_code_using::<generator::Storage>();
        let events = self.generate_code_using::<generator::Events>();
        let dispatch = self.generate_code_using::<generator::Dispatch>();
        let item_impls = self.generate_code_using::<generator::ItemImpls>();
        let cross_calling = self.generate_code_using::<generator::CrossCalling>();
        let metadata = self.generate_code_using::<generator::Metadata>();

        // 没用宏修饰的块, 即用户自行写的代码
        let non_ink_items = self
            .contract
            .module()
            .items()
            .iter()
            .filter_map(ir::Item::map_rust_item);

        // 最终的展开结果
        quote! {
            #( #attrs )*
            #vis mod #ident { // --> pub mod flipper {
                #env
                #storage
                #events
                #dispatch
                #item_impls
                #cross_calling
                #metadata
                #( #non_ink_items )* // 用户自己添加的代码被放在最后
            }
        }
    }
```

如上的代码, `#[ink_lang::contract]`宏实际上是整个代码生产的主入口, 进一步分成了几个子模块, 这里先了解下各个子模块的作用.

**env**, 我们使用`ink!`合约时, 总会用到一些来自链定义的类型, 要注意到`Substract`是一个链实现的通用框架, 它允许我们自行定义基础的类型, 这样同样的合约在实际运行中时, 一些类型的实现可能不同, 因此要定义好这些基础类型, 这就是`env`部分完成的工作.

**storage**, 在开发`ink!`合约时, 一个重要的部分就是合约的状态读写, 上面我们看到, 链实际只提供了`seal_get_storage`,`seal_set_storage`和`seal_clear_storage`三个导入接口, 这三个接口仅仅为我们实现了一个二进制map的数据模型, 为了方便我们使用`rust`类型进行合约开发, 这里做了很多工作.

**events**, 这里实现了`events`的定义.

**dispatch**, 这里实现了`call`和`deploy`的导出函数, 如上文所述, 这两个函数其实是整个合约执行的入口.

**item_impls**, 对于不同的message, 将会对应不同的实现, 这里定义了这两者的关联

**cross_calling**, 同绝大多数链一样, `ink!`合约中可以发起对其他合约的调用, 这是由`seal_call`接口实现的, 但是如上文所述, 这中间实际上存在着一个中间层, 要想完成合约间互调用, 在发起调用前, 要准备好调用的参数, 这里除了调用合约的AccountId之外, 还包括调用message的`Selector`以及参数的序列化, 也就是说, 要完成一次有效的跨合约调用, 需要发起调用的合约完全知道被调用合约的message接口定义, 因此, `ink!`鼓励用户采用直接引入其他合约包的方式实现跨合约调用, 也就是说, 如果一个合约以`ink-as-dependency`的特性被引入其他合约中, 其生成的代码将是一个可以跨合约调用的入口结构, **cross_calling**正是完成这里的逻辑.

**metadata**, 在编译合约时, 也会生成一个`metadata.json`的文件, 用来描述合约, 这个文件的生产函数就在这一块实现.

上面几方面涉及的代码很多, 我们这里先简单分析下**env**, 后续的分析会划分成单独的文章.

**env**实现为:

```rust
       let env = self.generate_code_using::<generator::Env>();
```

我们在generator下的env.rs中定义:

```rust
/// Generates code for the ink! environment of the contract.
#[derive(From)]
pub struct Env<'a> {
    contract: &'a ir::Contract,
}

impl GenerateCode for Env<'_> {
    // 生成代码
    fn generate_code(&self) -> TokenStream2 {
        // env配置信息, 这些由项目定义
        let env = self.contract.config().env();
        // storage修饰结构的标识符, 上面例子中即`Flipper`
        let storage_ident = self.contract.module().storage().ident();

        // 生成的代码
        quote! {
            // 为我们的主结构关联Env类型, 这样其他地方就可以关联到Env类型了
            impl ::ink_lang::ContractEnv for #storage_ident {
                type Env = #env;
            }

            type Environment = <#storage_ident as ::ink_lang::ContractEnv>::Env;

            // 一些定义, 这些都是必要的链类型
            type AccountId = <<#storage_ident as ::ink_lang::ContractEnv>::Env as ::ink_env::Environment>::AccountId;
            type Balance = <<#storage_ident as ::ink_lang::ContractEnv>::Env as ::ink_env::Environment>::Balance;
            type Hash = <<#storage_ident as ::ink_lang::ContractEnv>::Env as ::ink_env::Environment>::Hash;
            type Timestamp = <<#storage_ident as ::ink_lang::ContractEnv>::Env as ::ink_env::Environment>::Timestamp;
            type BlockNumber = <<#storage_ident as ::ink_lang::ContractEnv>::Env as ::ink_env::Environment>::BlockNumber;
        }
    }
}
```

对应生成的代码:

```rust
    impl ::ink_lang::ContractEnv for Flipper {
        type Env = ::ink_env::DefaultEnvironment;
    }
    type Environment = <Flipper as ::ink_lang::ContractEnv>::Env;
    type AccountId = <<Flipper as ::ink_lang::ContractEnv>::Env as ::ink_env::Environment>::AccountId;
    type Balance = <<Flipper as ::ink_lang::ContractEnv>::Env as ::ink_env::Environment>::Balance;
    type Hash = <<Flipper as ::ink_lang::ContractEnv>::Env as ::ink_env::Environment>::Hash;
    type Timestamp = <<Flipper as ::ink_lang::ContractEnv>::Env as ::ink_env::Environment>::Timestamp;
    type BlockNumber = <<Flipper as ::ink_lang::ContractEnv>::Env as ::ink_env::Environment>::BlockNumber;
```

这里这些基础类型是由宏生成的环境来决定的.

## ink!中的存储

下面我们分析`ink!`中的`storage`, 对于一个智能合约来说其存储是最重要的一部分, 我们先从`Contracts Pallet`的角度看`ink!`合约的底层存储结构是怎样的:

在`ink!`的文档中有一个很详细的图:

![kv](https://paritytech.github.io/ink-docs/img/kv.svg)

在合约的角度看, `Contracts Pallet`为我们提供了一个Key-Value数据库, 其中Key固定为256位, Value是一个字节串, 在合约层基于以下三个调用来使用:

- seal_set_storage Set存储
- seal_get_storage Get存储
- seal_clear_storage 删去指定Key的存储

在`Contracts Pallet`中, 每个合约只能操作合约实例(AccountId为标识)自身的Key-Value数据库, 因此整个合约的状态可以视为上面图示的DB, 但是我们在合约开发中很难直接使用这些接口.

为了简化开发, `ink!`采用了一个"storage 结构"来与合约状态DB建立一个映射关系, 这就是我们在合约中看到的`#[ink(storage)]`标记的结构, 如例子中:

```rust
    #[ink(storage)]
    pub struct Flipper {
        value: bool,
        value2: bool,
    }
```

这个之中`Flipper`就是这个结构, 是整个合约状态DB的的映射.

对于整个`ink!`合约, 这个结构体(被称为`storage`)扮演了整个合约代码的中枢, 上一篇文章中我们会看到这样的代码:

```rust
    impl ::ink_lang::ContractEnv for Flipper {
        type Env = ::ink_env::DefaultEnvironment;
    }
    type Environment = <Flipper as ::ink_lang::ContractEnv>::Env;
    type AccountId = <<Flipper as ::ink_lang::ContractEnv>::Env as ::ink_env::Environment>::AccountId;
```

这里`Flipper`作为`::ink_lang::ContractEnv`时, 负责关联`Env`类型, 这样的例子在整个合约中很常见, 我们分析展开后的代码时, 经常会见到`<<Flipper as X>::Y as Z>::func()`这样调用, 就是将`storage`结构作为中枢来表征其他逻辑关系.

在`ink!`中, 为了让我们可以很顺畅的使用`storage`结构, 因此, 合约执行时, 会先将合约状态db读入`storage`结构, 之后基于这个结构数据执行我们写的代码, 最后将`storage`结构写入合约状态db.

这就会引出一个问题, 我们的例子中合约状态整体很小, 读写的消耗不大, 因此按照上面流程是可以被接受的, 但是大部分合约的状态是很大的, 如一个典型的`ERC20`合约:

```rust
    /// A simple ERC-20 contract.
    #[ink(storage)]
    pub struct Erc20 {
        /// Total token supply.
        total_supply: Lazy<Balance>,
        /// Mapping from owner to number of owned token.
        balances: StorageHashMap<AccountId, Balance>,
        /// Mapping of the token amount which an account is allowed to withdraw
        /// from another account.
        allowances: StorageHashMap<(AccountId, AccountId), Balance>,
    }
```

如果每次都把所有人的Token信息都读出来, 那消耗过大了, 因此`ink!`引入了`Lazy`泛型, 被`Lazy`所包括的类型不会在合约执行初期就被读取出来, 这样就避免了大量的无异议的读写.

以上是`storage`结构的大致设计, 下面就是如何把`storage`结构与合约存储接口关联起来的问题了, 首先是所谓的合约状态DB, 这可以视作kv map, `ink!`中定义了`Layout`与`LayoutKey`来标志.

`Layout`与`LayoutKey`定义在`metadata`包中, 因为其实并不需要封装kv map, 只需要包装好链回调即可, 但是由于要生成metadata, 因此这里有必要包含实际存储的数据模型信息, 因此有了这两个定义.

```rust
/// Represents the static storage layout of an ink! smart contract.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, From, Serialize, Deserialize)]
#[serde(bound(
    serialize = "F::Type: Serialize, F::String: Serialize",
    deserialize = "F::Type: DeserializeOwned, F::String: DeserializeOwned"
))]
#[serde(rename_all = "camelCase")]
pub enum Layout<F: Form = MetaForm> {
    /// An encoded cell.
    ///
    /// This is the only leaf node within the layout graph.
    /// All layout nodes have this node type as their leafs.
    ///
    /// This represents the encoding of a single cell mapped to a single key.
    Cell(CellLayout<F>),
    /// A layout that hashes values into the entire storage key space.
    ///
    /// This is commonly used by ink! hashmaps and similar data structures.
    Hash(HashLayout<F>),
    /// An array of associated storage cells encoded with a given type.
    ///
    /// This can also represent only a single cell.
    Array(ArrayLayout<F>),
    /// A struct layout with fields of different types.
    Struct(StructLayout<F>),
    /// An enum layout with a discriminant telling which variant is layed out.
    Enum(EnumLayout<F>),
}

/// A pointer into some storage region.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, From)]
pub struct LayoutKey {
    key: [u8; 32],
}
```

`LayoutKey`比较直接, 这里先略过, 可以看到`Layout`是一个泛型的`enum`, `F`是携带供`metadata`的类型标志.

`Layout`的细节会在**metadata**生成过程中涉及, 这里我们先了解下意义.

我们从另外一个方向看, 现在我们知道, 所有状态都由`storage`结构类型来定义, 那么这个结构类型必须满足两点:

- 1. `storage`结构类型必须**关联**其以`Layout`表征的数据模型
- 2. `storage`结构类型必须**实现**与合约状态DB上对应数据的双向转换

在`ink!`中, 对于`storage`结构类型, 前者通过实现`ink_storage::traits::StorageLayout` trait完成, 后者通过实现`ink_storage::traits::SpreadLayout` trait完成.

实际上, 不仅仅是`storage`结构类型, 任何一个类型, 只要实现了上述两个trait, 就可以作为合约的存储, 当然, 在`ink!`中, 只有一个`storage`结构类型与合约状态DB进行映射, 当然, 这个`storage`结构类型也可以包含其他类型, 这些类型就要满足上述的条件.

先来看一下`StorageLayout`:

```rust
/// Implemented by types that have a storage layout.
pub trait StorageLayout {
    /// Returns the static storage layout of `Self`.
    ///
    /// The given key pointer is guiding the allocation of static fields onto
    /// the contract storage regions.
    fn layout(key_ptr: &mut KeyPtr) -> Layout;
}
```

这个trait很简单, 只要实现`layout`函数返回一个`Layout`即可, 这个`Layout`主要提供合约状态DB的数据模型, 这里主要作为生成metadata使用.

重点是`SpreadLayout`, 在理解`SpreadLayout`之前我们先来思考一下应该怎样实现与合约状态DB上对应数据的双向转换.

我们使用的结构类型可以通过组合形成一个多层级的树状结构, 但是合约状态DB只是单层的KV-Map, 因此需要通过key的分层来对应结构类型的树状结构.

`ink!`并没有规定该怎么样建立这种key的联系, 但是为通用的几种数据结构规范了一系列生成key的方法, 在metadata中标识了生成key的方法, 便于第三方去读取.

另外一方面就是存储的数据本身, 对于绝大多数类型可以直接通过`parity-scale-codec`来序列化成二进制串, 当然, 对于一些集合类型, 也可以将多个项打包成KV map中的一项. 不同的方式各有优劣, 开发者可以根据特定情况来选择.

现在我们来看一下`SpreadLayout`:

```rust
/// Types that can be stored to and loaded from the contract storage.
pub trait SpreadLayout {
    /// The footprint of the type.
    ///
    /// This is the number of adjunctive cells the type requires in order to
    /// be stored in the contract storage with spread layout.
    ///
    /// # Examples
    ///
    /// An instance of type `i32` requires one storage cell, so its footprint is
    /// 1. An instance of type `(i32, i32)` requires 2 storage cells since a
    /// tuple or any other combined data structure always associates disjunctive
    /// cells for its sub types. The same applies to arrays, e.g. `[i32; 5]`
    /// has a footprint of 5.
    const FOOTPRINT: u64;

    /// Indicates whether a type requires deep clean-up of its state meaning that
    /// a clean-up routine has to decode an entity into an instance in order to
    /// eventually recurse upon its tear-down.
    /// This is not required for the majority of primitive data types such as `i32`,
    /// however types such as `storage::Box` that might want to forward the clean-up
    /// procedure to their inner `T` require a deep clean-up.
    ///
    /// # Note
    ///
    /// The default is set to `true` in order to have correctness by default since
    /// no type invariants break if a deep clean-up is performed on a type that does
    /// not need it but performing a shallow clean-up for a type that requires a
    /// deep clean-up would break invariants.
    /// This is solely a setting to improve performance upon clean-up for some types.
    const REQUIRES_DEEP_CLEAN_UP: bool = true;

    /// Pulls an instance of `Self` from the contract storage.
    ///
    /// The key pointer denotes the position where the instance is being pulled
    /// from within the contract storage
    ///
    /// # Note
    ///
    /// This method of pulling is depth-first: Sub-types are pulled first and
    /// construct the super-type through this procedure.
    fn pull_spread(ptr: &mut KeyPtr) -> Self;

    /// Pushes an instance of `Self` to the contract storage.
    ///
    /// - Tries to spread `Self` to as many storage cells as possible.
    /// - The key pointer denotes the position where the instance is being pushed
    /// to the contract storage.
    ///
    /// # Note
    ///
    /// This method of pushing is depth-first: Sub-types are pushed before
    /// their parent or super type.
    fn push_spread(&self, ptr: &mut KeyPtr);

    /// Clears an instance of `Self` from the contract storage.
    ///
    /// - Tries to clean `Self` from contract storage as if `self` was stored
    ///   in it using spread layout.
    /// - The key pointer denotes the position where the instance is being cleared
    ///   from the contract storage.
    ///
    /// # Note
    ///
    /// This method of clearing is depth-first: Sub-types are cleared before
    /// their parent or super type.
    fn clear_spread(&self, ptr: &mut KeyPtr);
}
```

首先需要注意两个常量: `FOOTPRINT`与`REQUIRES_DEEP_CLEAN_UP`, 代码中的注释解释的很清楚, 接下来就是三个接口, 分别是读, 写和清理, 简单的看了一下这些基本结构之后, 我们可以转回去看一下存储相关的宏展开代码:

```rust
impl GenerateCode for Storage<'_> {
    fn generate_code(&self) -> TokenStream2 {
        let storage_span = self.contract.module().storage().span();
        let access_env_impls = self.generate_access_env_trait_impls();
        let storage_struct = self.generate_storage_struct();
        let use_emit_event = if self.contract.module().events().next().is_some() {
            // Required to allow for `self.env().emit_event(..)` in messages and constructors.
            Some(quote! { use ::ink_lang::EmitEvent as _; })
        } else {
            None
        };
        let cfg = self.generate_code_using::<generator::CrossCallingConflictCfg>();
        quote_spanned!(storage_span =>
            #access_env_impls
            #storage_struct

            #cfg
            const _: () = {
                // Used to make `self.env()` available in message code.
                #[allow(unused_imports)]
                use ::ink_lang::{
                    Env as _,
                    StaticEnv as _,
                };
                #use_emit_event
            };
        )
    }
}
```

这里首先看下`#access_env_impls`这一部分, 实际上这更多是建立了`storage`结构类型与`EnvAccess`类型的关联, 严格意义上属于面向env的代码.

```rust
    fn generate_access_env_trait_impls(&self) -> TokenStream2 {
        // storage修饰结构的标识符, 上面例子中即`Flipper`
        let storage_ident = &self.contract.module().storage().ident();

        // 就是 #[cfg(not(feature = "ink-as-dependency"))]
        let cfg = self.generate_code_using::<generator::CrossCallingConflictCfg>();

        quote! {
            // `const _: () = {`这样的写法就是适配`#cfg`
            #cfg 
            const _: () = {
                impl<'a> ::ink_lang::Env for &'a #storage_ident {
                    type EnvAccess = ::ink_lang::EnvAccess<'a, <#storage_ident as ::ink_lang::ContractEnv>::Env>;

                    fn env(self) -> Self::EnvAccess {
                        Default::default()
                    }
                }

                impl<'a> ::ink_lang::StaticEnv for #storage_ident {
                    type EnvAccess = ::ink_lang::EnvAccess<'static, <#storage_ident as ::ink_lang::ContractEnv>::Env>;

                    fn env() -> Self::EnvAccess {
                        Default::default()
                    }
                }
            };
        }
    }
```

生成的代码:

```rust
    #[cfg(not(feature = "ink-as-dependency"))]
    const _: () = {
        impl<'a> ::ink_lang::Env for &'a Flipper {
            type EnvAccess =
                ::ink_lang::EnvAccess<'a, <Flipper as ::ink_lang::ContractEnv>::Env>;
            fn env(self) -> Self::EnvAccess {
                Default::default()
            }
        }
        impl<'a> ::ink_lang::StaticEnv for Flipper {
            type EnvAccess =
                ::ink_lang::EnvAccess<'static, <Flipper as ::ink_lang::ContractEnv>::Env>;
            fn env() -> Self::EnvAccess {
                Default::default()
            }
        }
    };
```

重点关注下`storage`结构, 需要注意的是, `ink!`为了实现跨合约调用, 所以当合约作为依赖引入时, 将会展开成不同的实现, 这里我们先考虑合约本身编译时展开的代码:

```rust
    /// Generates the storage struct definition.
    fn generate_storage_struct(&self) -> TokenStream2 {
        let storage = self.contract.module().storage();
        let span = storage.span();
        let ident = &storage.ident();
        let attrs = &storage.attrs();
        let fields = storage.fields();
        let cfg = self.generate_code_using::<generator::CrossCallingConflictCfg>();
        quote_spanned!( span =>
            #cfg
            #(#attrs)*
            #[cfg_attr(
                feature = "std",
                derive(::ink_storage::traits::StorageLayout)
            )]
            #[derive(::ink_storage::traits::SpreadLayout)]
            #[cfg_attr(test, derive(Debug))]
            pub struct #ident {
                #( #fields ),*
            }
        )
    }
```

这个如果简单替换应该是这样的:

```rust
            #[cfg(not(feature = "ink-as-dependency"))]
            #[cfg_attr(
                feature = "std",
                derive(::ink_storage::traits::StorageLayout)
            )]
            #[derive(::ink_storage::traits::SpreadLayout)]
            #[cfg_attr(test, derive(Debug))]
            pub struct Flipper {
                value: bool,
                value2: bool,
            }
```

当然这里会进一步展开, 主要是`StorageLayout`和`SpreadLayout`的展开, 我们按顺序先看`SpreadLayout`部分展开之后的结果:

```rust
    // 原本的结构定义
    #[cfg(not(feature = "ink-as-dependency"))]
    pub struct Flipper {
        value: bool,
        value2: bool,
    }

    // 这一部分是`SpreadLayout`的实现
    const _: () = {
        impl ::ink_storage::traits::SpreadLayout for Flipper {
            #[allow(unused_comparisons)]

            // 定义`FOOTPRINT`, 这里我们format了一下代码,
            const FOOTPRINT: u64 = [
                (
                    ( 0u64 + <bool as ::ink_storage::traits::SpreadLayout>::FOOTPRINT )
                           + <bool as ::ink_storage::traits::SpreadLayout>::FOOTPRINT
                ),
                0u64, // 如果上面值小于0, 则为零, 这是一种典型的编译期编程手法
            ][
                (
                    (
                        ( 0u64 + <bool as ::ink_storage::traits::SpreadLayout>::FOOTPRINT )
                               + <bool as ::ink_storage::traits::SpreadLayout>::FOOTPRINT
                    )
                    < 0u64 // 这里实际上是如果上面值小于0, 则为零, 这是一种典型的编译期编程手法
                ) as usize
            ];

            // 定义`REQUIRES_DEEP_CLEAN_UP`, 这里改写了一下方式, 实际上就是所有子类型的或
            const REQUIRES_DEEP_CLEAN_UP: bool = (
                    (
                           false 
                        || <bool as ::ink_storage::traits::SpreadLayout>::REQUIRES_DEEP_CLEAN_UP
                        || <bool as ::ink_storage::traits::SpreadLayout>::REQUIRES_DEEP_CLEAN_UP
                    )
            );

            // 读取, 对于每项分别读取, 返回`Flipper`
            fn pull_spread(__key_ptr: &mut ::ink_storage::traits::KeyPtr) -> Self {
                Flipper {
                    value: <bool as ::ink_storage::traits::SpreadLayout>::pull_spread(
                        __key_ptr,
                    ),
                    value2: <bool as ::ink_storage::traits::SpreadLayout>::pull_spread(
                        __key_ptr,
                    ),
                }
            }

            // 写入实现, 注意这里通过匹配的方式实现了对每一项分别调用`push_spread`
            fn push_spread(&self, __key_ptr: &mut ::ink_storage::traits::KeyPtr) {
                match self {
                    Flipper {
                        value: __binding_0,
                        value2: __binding_1,
                    } => {
                        {
                            ::ink_storage::traits::SpreadLayout::push_spread(
                                __binding_0,
                                __key_ptr,
                            );
                        }
                        {
                            ::ink_storage::traits::SpreadLayout::push_spread(
                                __binding_1,
                                __key_ptr,
                            );
                        }
                    }
                }
            }

            // 删除实现
            fn clear_spread(&self, __key_ptr: &mut ::ink_storage::traits::KeyPtr) {
                match self {
                    Flipper {
                        value: __binding_0,
                        value2: __binding_1,
                    } => {
                        {
                            ::ink_storage::traits::SpreadLayout::clear_spread(
                                __binding_0,
                                __key_ptr,
                            );
                        }
                        {
                            ::ink_storage::traits::SpreadLayout::clear_spread(
                                __binding_1,
                                __key_ptr,
                            );
                        }
                    }
                }
            }
        }
    };
```

需要注意的是`pull_spread`和`push_spread`中的`__key_ptr`参数, 上文提到, 需要通过key的分层来对应结构类型的树状结构, 这里的`__key_ptr`参数就是一个用于表示key的上下文信息, 每次基于`__key_ptr`生成一个新的key值同时也会更新上下文信息, 值得注意的是, `ink!`中key分配的值是和`push_spread`中的调用顺序相关的, 也就是说, 如果我们的合约存储的类型调换了顺序, 那么对应生产代码所指派的key也会变化. 这就意味着`pull_spread`和`push_spread`必须一一对应, 进一步每一级的`pull_spread`和`push_spread`也要一一对应, 否者读取的数据会出现错误.

上面`SpreadLayout`的展开在`storage::derive::spread_layout_derive`中:

```rust
/// Derives `ink_storage`'s `SpreadLayout` trait for the given `struct` or `enum`.
pub fn spread_layout_derive(mut s: synstructure::Structure) -> TokenStream2 {
    s.bind_with(|_| synstructure::BindStyle::Move)
        .add_bounds(synstructure::AddBounds::Generics)
        .underscore_const(true);
    match s.ast().data {
        syn::Data::Struct(_) => spread_layout_struct_derive(&s),
        syn::Data::Enum(_) => spread_layout_enum_derive(&s),
        _ => {
            panic!(
                "cannot derive `SpreadLayout` or `PackedLayout` for Rust `union` items"
            )
        }
    }
}
```

这里对于`enum`与`struct`进行了分别处理, 主要的逻辑就是根据结构中每一项生成对应的代码.

接下来是`StorageLayout`:

```rust
    const _: () = {
        impl ::ink_storage::traits::StorageLayout for Flipper {
            fn layout(
                __key_ptr: &mut ::ink_storage::traits::KeyPtr,
            ) -> ::ink_metadata::layout::Layout {
                ::ink_metadata::layout::Layout::Struct(
                    ::ink_metadata::layout::StructLayout::new(<[_]>::into_vec(box [
                        ::ink_metadata::layout::FieldLayout::new(
                            Some("value"),
                            <bool as ::ink_storage::traits::StorageLayout>::layout(
                                __key_ptr,
                            ),
                        ),
                        ::ink_metadata::layout::FieldLayout::new(
                            Some("value2"),
                            <bool as ::ink_storage::traits::StorageLayout>::layout(
                                __key_ptr,
                            ),
                        ),
                    ])),
                )
            }
        }
    };
```

这里的逻辑中就是拼接出一个`Layout`, 注意由于上文提到的, 每一项的的顺序是严格有序的, 因此这里生成的metadata是有序的数组.

到此为止就是存储相关代码的宏展开流程, 这里我们同时看一下生成的`storage`结构是怎样被使用的, 这里我们先看一下生成的合约执行代码, 这一部分的详细分析可以参照后面的文档, 我们先来梳理存储相关的逻辑:

展开的代码中会调用`execute_message_mut`来执行, 这里我们先只看会变更状态的exec message, 这里将我们所写下的对于message函数以闭包的形式传入调用:

```rust
            impl ::ink_lang::Execute for __ink_MessageDispatchEnum {
                // 执行合约时, 会调用这里的代码, 这里我们省略了部分无关的代码
                fn execute(
                    self,
                ) -> ::core::result::Result<(), ::ink_lang::DispatchError>
                {
                    match self {
                        ...
                        Self::__ink_Message_0x633aa551() => {
                            // 实际的执行调用
                            ::ink_lang::execute_message_mut::<
                                <Flipper as ::ink_lang::ContractEnv>::Env,
                                __ink_Msg<[(); 1369782883usize]>,
                                _,
                            >(
                                ...
                                move |state: &mut Flipper| {
                                    // 这里最终调用了我们所开发的message函数, 注意state就是已经映射的`storage`结构
                                    < __ink_Msg < [() ; 1369782883usize]> as::ink_lang::MessageMut>::CALLABLE(state ,())
                                },
                            )
                        }
                        ...
                    }
                }
            }
```

我们看下`::ink_lang::execute_message_mut`:

```rust
/// Executes the given `&mut self` message closure.
///
/// # Note
///
/// The closure is supposed to already contain all the arguments that the real
/// message requires and forwards them.
#[inline]
#[doc(hidden)]
pub fn execute_message_mut<E, M, F>(
    accepts_payments: AcceptsPayments,
    enables_dynamic_storage_allocator: EnablesDynamicStorageAllocator,
    f: F,
) -> Result<()>
where
    E: Environment, // `E`是`Env`, 就是上文提到的Env信息
    M: MessageMut, // `M`相当于message对应的类函数类型
    F: FnOnce(&mut <M as FnState>::State) -> <M as FnOutput>::Output, // `F`就是闭包类型, 直接关联着调用具体的逻辑
{
    // 这里略去一些执行相关的代码

    // 注意这里的root_key, 所有存储的key最终基于这个
    let root_key = Key::from([0x00; 32]);

    // 这里就是`storage`结构的读取, 可以看到, 这里直接调用`pull_spread_root`, 这个函数会调用生成的`pull_spread`
    let mut state =
        ManuallyDrop::new(pull_spread_root::<<M as FnState>::State>(&root_key));

    // 我们开发的message处理函数就是这里调用
    let result = f(&mut state);

    // 最后调用`push_spread_root`来调用生成的`push_spread`把状态写回
    push_spread_root::<<M as FnState>::State>(&state, &root_key);

    // 这里略去一些执行相关的代码

    // 处理返回
    if TypeId::of::<<M as FnOutput>::Output>() != TypeId::of::<()>() {
        ink_env::return_value::<<M as FnOutput>::Output>(ReturnFlags::default(), &result)
    }
    Ok(())
}
```

以上就是合约中处理存储的流程, 基于此, 我们注意到`Lazy`的重要性, 因为合约执行时会"无脑的"读写存储结构中的所有信息, 因此`Lazy`类型非常重要.

## ink!中的Event

## ink!中的Message处理

## 关于跨合约调用的注记
