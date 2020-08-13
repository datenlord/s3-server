#[allow(unused_macros)]
macro_rules! cfg_rt_tokio{
    {$($item:item)*}=>{
        $(
            #[cfg(feature = "rt-tokio")]
            $item
        )*
    }
}
