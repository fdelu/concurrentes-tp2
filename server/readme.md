# Servidor

Este módulo se encarga de implementar todas las funcionalidades del servidor, siendo las más importantes:
- Comunicación con las cafeteras locales
- Comunicación y organización entre servidores de diferentes cafeterías
- Agregado y descuento de puntos a los clientes

## Algoritmos distribuidos implementados

Para garantizar el correcto funcionamiento de los servidores en un ambiente inestable, en el que los servidores se
pueden desconectar de la red continuamente, se implementaron los siguientes algoritmos

### Algoritmo distribuido - acceso a secciones críticas
Cada cuenta de usuario representa una sección crítica,  que se protege con un Mutex distribuido diferente.
Cada sección crítica se identifica por el id del usuario.
### Commit en dos fases - modificación de una sección crítica
Una vez que obtiene permiso para acceder a la sección
crítica, se utiliza este algoritmo para garantizar que todos los servidores de la red realicen el agregado y descuento
de puntos correctamente.

## Políticas

Como la definición de los algoritmos no especifica el comportamiento en muchas situaciones reales, se definió una serie
de políticas que permiten el correcto funcionamiento de la aplicación en caso de fallos en la red

### Detección de servidores caídos
Este mecanismo se implementó internamente, en el algoritmo de mutex distribuido, de la siguiente manera:

Cuando un servidor quiere realizar alguna acción que implica comunicación con los otros servidores, debe tomar un lock.
Esto implica enviar un mensaje de `Request` a todos los servidores solicitando permiso.
El resto de servidores contesta siempre con un paquete de `Acknowledge`, y seguidamente con un `Ok` en caso de otorgar
permiso.
El servidor emisor, entonces, espera un determinado tiempo para recibir los acknowledges. Si alguno de los servidores
no contesta, significa que está desconectado. Por lo tanto, a partir de ese momento ya no se le enviarán mensajes, ni
se esperará a su respuesta en las votaciones de `Prepare` (del algoritmo de commit), ni se esperará el `Ok` para el
acceso a secciones críticas.

Por otro lado, si al enviar el `Request`, ningún servidor contesta con acknowledge, significa que el emisor se encuentra
desconectado.

### Abort implícito de transacción

Si el emisor de un `Prepare` no envía un `Commit` o `Rollback` luego de un período de tiempo, se considera que la transacción
debe ser abortada.

### Reconexión de servidores
Cuando un servidor detecta que se encuentra desconectado, comienza a enviar mensajes de `SyncRequest`. Cuando se conecte
nuevamente, el resto de servidores responderán con un `SyncResponse`, en el cual se envía la base de datos actualizada.
Este mensaje permite
- Obtener los cambios producidos en la red mientras el servidor se encontraba desconectado
- Comunicar al resto de servidores que la conexión se ha restablecido

## Modelo de concurrencia

El módulo fue implementado casi en su totalidad utilizando herramientas del modelo de actores, con la ayuda del crate
[actix](https://crates.io/crates/actix).

### Actores

#### DistMutex
Implementa el algoritmo distribuido para el acceso a secciones críticas. Cada sección crítica es un mutex diferente.

Sus mensajes principales son `Acquire` y `Release`, para ingresar y salir de una sección crítica.
Una forma más segura de utilizarlo, es con el mensaje `DoWithLock`, que adquiere el lock, ejecuta un closure, y lo
libera.

Internamente, el actor se comunica con los mutexes de otros servidores con los mensajes `Ack` y `Ok`.

### TwoPhaseCommit
Implementa el algoritmo de transacciones en dos fases. Existe uno solo por servidor, y posee la base de datos como atributo
interno.

Sus mensajes principales son `Prepare`, `Commit` y `Rollback` para ejecutar la primera y segunda fase del algoritmo.

### PacketDispatcher
Comunica a los actores del servidor local con los actores de otros servidores. Existe uno solo por servidor.

Sus mensajes principales son `Broadcast` y `Send`, para enviar un mensaje a todos los servidores, o a uno en específico.

Por otro lado, gestiona internamente el reintento de agregado de puntos, en caso de que el servidor se encuentre sin
conexión.

### Client Connections
Maneja las conexiones con el cliente. Este actor recive ordenes de la cafetera a traves de `ConnectionHandler`, las procesa y envia la información correspondiente a `PacketDispatcher`. Tambien se encarga de contestarle a la cafetera cuando es necesario y producir los errores correspondientes si los hay. Este actor se maneja con los tipos de paquetes que maneja la cafetera asi que para más información sobre este protocolo ver su [readme.md](https://github.com/concurrentes-fiuba/2022-2c-tp2-rostov/blob/main/coffee_maker/readme.md).

### ConnectionHandler
Simplifica la comunicación, a bajo nivel, entre diferentes servidores.
Expone una interfaz, con mensajes, que se asemeja mucho a la comunicación UDP: `SendPacket` para enviar un paquete a una
dirección específica, y `RecvPacket` para recibir un paquete desde cualquier servidor.
Para garantizar la entrega confiable de la información, se implementó utilizando TCP

## Configuración

La cafetera recibe como parametro un archivo de configuración tipo JSON. Los parametros son.

`server_ip`: El ip para este el servidor que se va a ejecutar.

`server_port`: El puerto por donde se va a comunicar con otros servidores.

`client_port`: El puerto por donde se va a comunicar con las cafeteras.

`servers`: Una lista de ips de los servidores en la red.

`add_points_interval_ms`: Intervalo entre intentos de agregar puntos.

`logs`: Dentro se pueden definir donde ira el archivo que guarde los logs, en nivel de logs de este archivo y el nivel de logs de los mostrados por consola.

## Comandos

### Ejecución

```
cargo run [config file]
```

### Tests

```
cargo test
RUSTFLAGS="--cfg mocks" cargo test
```

### Documentación

```
cargo doc --open
```

## Diagramas de secuencia
En [este link](https://lucid.app/lucidchart/fe4201b8-dd5d-42d6-b5bb-24b74f79abf9/edit?viewport_loc=-2140%2C-782%2C8308%2C4238%2CesH_8SpTMZLH&invitationId=inv_5c935c5e-dfe6-4482-9fbc-cc2dfc6181d1) se pueden encontrar algunos diagramas de secuencia del servidor.
