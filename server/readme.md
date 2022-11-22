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

### ConnectionHandler
Simplifica la comunicación, a bajo nivel, entre diferentes servidores.
Expone una interfaz, con mensajes, que se asemeja mucho a la comunicación UDP: `SendPacket` para enviar un paquete a una
dirección específica, y `RecvPacket` para recibir un paquete desde cualquier servidor.
Para garantizar la entrega confiable de la información, se implementó utilizando TCP