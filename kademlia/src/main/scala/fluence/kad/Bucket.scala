package fluence.kad

import cats.{Applicative, MonadError}
import cats.data.StateT
import cats.syntax.eq._
import cats.syntax.flatMap._
import cats.syntax.applicativeError._
import cats.syntax.applicative._

import scala.language.higherKinds

case class Bucket(maxSize: Int, contacts: Seq[Contact] = List.empty) {
  lazy val isFull: Boolean = contacts.size >= maxSize
}

object Bucket {

  /**
    * Returns the bucket state
    * @tparam X StateT effect
    * @return The bucket
    */
  def bucket[X[_]: Applicative]: StateT[X, Bucket, Bucket] = StateT.get[X, Bucket]

  /**
    * Lookups the bucket for a particular key, not making any external request
    * @param key Contact key
    * @tparam X StateT effect
    * @return optional found contact
    */
  def find[X[_]: Applicative](key: Key): StateT[X, Bucket, Option[Contact]] =
    bucket[X].map{b =>
      b.contacts.find(_.key === key)
    }

  /**
    * Performs bucket update.
    *
    * Whenever a node receives a communication from another, it updates the corresponding bucket.
    * If the contact already exists, it is _moved_ to the (head) of the bucket.
    * Otherwise, if the bucket is not full, the new contact is _added_ at the (head).
    * If the bucket is full, the node pings the contact at the (end) of the bucket's list.
    * If that least recently seen contact fails to respond in an (unspecified) reasonable time,
    * it is _dropped_ from the list, and the new contact is added at the (head).
    * Otherwise the new contact is _ignored_ for bucket updating purposes.
    *
    * @param contact Contact to check and update
    * @param ping Ping function
    * @param ME Monad error for StateT effect
    * @tparam X StateT effect
    * @return updated Bucket
    */
  def update[X[_]](contact: Contact, ping: Contact => X[Contact])(implicit ME: MonadError[X, Throwable]): StateT[X, Bucket, Unit] = {
    bucket[X].flatMap {b =>
      find[X](contact.key).flatMap {
          case Some(c) =>
            // put contact on top
            StateT set b.copy(contacts = contact +: b.contacts.filterNot(_.key == c.key))
          case None if b.isFull =>
            // ping last, if pong, put last on top and drop contact, if not, drop last and put contact on top
            StateT setF ping(b.contacts.last).attempt.flatMap {
              case Left(_) =>
                b.copy(contacts = contact +: b.contacts.dropRight(1)).pure
              case Right(updatedLastContact) =>
                b.copy(contacts = updatedLastContact +: b.contacts.dropRight(1)).pure
            }
          case None =>
            // put contact on top
            StateT set b.copy(contacts = contact +: b.contacts)
      }
    }
  }

}