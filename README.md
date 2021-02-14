# harvest(WIP)


1. server 监听通过RPC请求将数据写入本地db,然后启动收集日志任务，数据包括过滤，输出，收集的pod
2. input 查找当前全名空间的日志文件，并生成元数据写入本地db
3. parser 解析过滤数据规则
4. output 输出


server code

    let server= Server::new(address);
    server.run();


input code
1.实现目录的wacth,如果日志文件有变动，则自动添加进任务
2.
let input = Input::new(namespace,pod_name)-> Result<Vec<Row>>; --> namespace eg: kube-system coredns 0.log,1.log,2.log

    input.collect_by_asc() 从小读到大?


取pod状态的containerID
status:
containerStatuses:
- containerID: docker://ba6c7c04e50f7e1aa50f470610c08caa312bd8fb84c51d2b340122e8d90a44f0

然后查找docker目录下的ba6c7c04e50f7e1aa50f470610c08caa312bd8fb84c51d2b340122e8d90a44f0/*.log文件
例如 /data/docker/containers/ba6c7c04e50f7e1aa50f470610c08caa312bd8fb84c51d2b340122e8d90a44f0/ba6c7c04e50f7e1aa50f470610c08caa312bd8fb84c51d2b340122e8d90a44f0-json.log
这里后缀名为-json.log表示docker使用json日志
