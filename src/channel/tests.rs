use async_ringbuf::AsyncHeapRb;
use flatty::{
    make_flat,
    portable::{le, NativeCast},
    FlatVec,
};
use futures::join;

use super::{read::MsgReadError, MsgReader, MsgWriter};

#[make_flat(sized = false, portable = true)]
enum TestMsg {
    A,
    B(le::I32),
    C(FlatVec<le::I32, le::U16>),
}

#[async_std::test]
async fn test() {
    const MAX_SIZE: usize = 32;
    let (prod, cons) = AsyncHeapRb::<u8>::new(17).split();
    join!(
        async move {
            let mut writer = MsgWriter::<TestMsg, _>::new(prod, MAX_SIZE);

            writer.init_msg(&TestMsgDyn::A).unwrap().write().await.unwrap();

            writer.init_msg(&TestMsgDyn::B(123456)).unwrap().write().await.unwrap();

            writer
                .init_msg(&TestMsgDyn::C(vec![0, 1, 2, 3, 4, 5, 6]))
                .unwrap()
                .write()
                .await
                .unwrap();
        },
        async move {
            let mut reader = MsgReader::<TestMsg, _>::new(cons, MAX_SIZE);

            {
                let guard = reader.read_msg().await.unwrap();
                match guard.as_ref() {
                    TestMsgRef::A => (),
                    _ => panic!(),
                }
            }

            {
                let guard = reader.read_msg().await.unwrap();
                match guard.as_ref() {
                    TestMsgRef::B(x) => assert_eq!(x.to_native(), 123456),
                    _ => panic!(),
                }
            }

            {
                let guard = reader.read_msg().await.unwrap();
                match guard.as_ref() {
                    TestMsgRef::C(v) => {
                        let vn = v.iter().map(|x| x.to_native()).collect::<Vec<_>>();
                        assert_eq!(&vn, &[0, 1, 2, 3, 4, 5, 6]);
                    }
                    _ => panic!(),
                }
            }

            match reader.read_msg().await.err().unwrap() {
                MsgReadError::Eof => (),
                _ => panic!(),
            }
        },
    );
}