# harvest(WIP)


pod info 
    state:  ready running stopped



取pod状态的containerID
status:
containerStatuses:
- containerID: docker://ba6c7c04e50f7e1aa50f470610c08caa312bd8fb84c51d2b340122e8d90a44f0

然后查找docker目录下的ba6c7c04e50f7e1aa50f470610c08caa312bd8fb84c51d2b340122e8d90a44f0/*.log文件
例如 /data/docker/containers/ba6c7c04e50f7e1aa50f470610c08caa312bd8fb84c51d2b340122e8d90a44f0/ba6c7c04e50f7e1aa50f470610c08caa312bd8fb84c51d2b340122e8d90a44f0-json.log
这里后缀名为-json.log表示docker使用json日志



## other
1 apiServer --post /task--> harvest {"service_name":"example_service","pods":{"node":"node1","pod":"example","ips":["127.0.0.1"]}}



harvest <---sse---> apiServer {"service_name":"example_service","pods":{"node":"node1","pod":"example","ips":["127.0.0.1"]},"upload":true,"output":"$kafka_uri","filter":""}
