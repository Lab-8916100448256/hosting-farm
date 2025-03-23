import React, { useState, useEffect } from 'react';

interface Invitation {
  pid: string;
  team: {
    pid: string;
    name: string;
  };
  role: string;
  invited_by: {
    name: string;
    email: string;
  };
  created_at: string;
}

const UserInvitations: React.FC = () => {
  const [invitations, setInvitations] = useState<Invitation[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [actionStatus, setActionStatus] = useState<{
    type: 'success' | 'error';
    message: string;
  } | null>(null);

  useEffect(() => {
    const fetchInvitations = async () => {
      try {
        const token = localStorage.getItem('token');
        if (!token) {
          throw new Error('No authentication token found');
        }

        const response = await fetch('/api/teams/invitations', {
          headers: {
            'Authorization': `Bearer ${token}`
          }
        });

        if (!response.ok) {
          throw new Error('Failed to fetch invitations');
        }

        const data = await response.json();
        setInvitations(data);
      } catch (err) {
        setError(err instanceof Error ? err.message : 'An error occurred while fetching invitations');
      } finally {
        setIsLoading(false);
      }
    };

    fetchInvitations();
  }, []);

  const handleAcceptInvitation = async (invitationPid: string) => {
    try {
      setActionStatus(null);
      const token = localStorage.getItem('token');
      if (!token) {
        throw new Error('No authentication token found');
      }

      const response = await fetch(`/api/teams/invitations/${invitationPid}/accept`, {
        method: 'POST',
        headers: {
          'Authorization': `Bearer ${token}`
        }
      });

      if (!response.ok) {
        throw new Error('Failed to accept invitation');
      }

      // Remove the invitation from the list
      setInvitations(invitations.filter(inv => inv.pid !== invitationPid));
      setActionStatus({
        type: 'success',
        message: 'Invitation accepted successfully! You can now access the team.'
      });
    } catch (err) {
      setActionStatus({
        type: 'error',
        message: err instanceof Error ? err.message : 'An error occurred while accepting the invitation'
      });
    }
  };

  const handleRejectInvitation = async (invitationPid: string) => {
    try {
      setActionStatus(null);
      const token = localStorage.getItem('token');
      if (!token) {
        throw new Error('No authentication token found');
      }

      const response = await fetch(`/api/teams/invitations/${invitationPid}/reject`, {
        method: 'POST',
        headers: {
          'Authorization': `Bearer ${token}`
        }
      });

      if (!response.ok) {
        throw new Error('Failed to reject invitation');
      }

      // Remove the invitation from the list
      setInvitations(invitations.filter(inv => inv.pid !== invitationPid));
      setActionStatus({
        type: 'success',
        message: 'Invitation rejected successfully.'
      });
    } catch (err) {
      setActionStatus({
        type: 'error',
        message: err instanceof Error ? err.message : 'An error occurred while rejecting the invitation'
      });
    }
  };

  if (isLoading) {
    return <div className="text-center py-10">Loading invitations...</div>;
  }

  if (error) {
    return (
      <div className="bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded mb-4" role="alert">
        <span className="block sm:inline">{error}</span>
      </div>
    );
  }

  return (
    <div>
      <h1 className="text-2xl font-bold mb-6">Team Invitations</h1>
      
      {actionStatus && (
        <div 
          className={`${
            actionStatus.type === 'success' ? 'bg-green-100 border-green-400 text-green-700' : 'bg-red-100 border-red-400 text-red-700'
          } px-4 py-3 rounded mb-4`} 
          role="alert"
        >
          <span className="block sm:inline">{actionStatus.message}</span>
        </div>
      )}
      
      {invitations.length === 0 ? (
        <div className="bg-gray-100 p-6 rounded-lg text-center">
          <p className="text-gray-700">You don't have any pending team invitations.</p>
        </div>
      ) : (
        <div className="bg-white shadow overflow-hidden sm:rounded-lg">
          <ul className="divide-y divide-gray-200">
            {invitations.map((invitation) => (
              <li key={invitation.pid} className="p-4">
                <div className="flex justify-between items-start">
                  <div>
                    <h3 className="text-lg font-medium text-gray-900">
                      Invitation to join <span className="font-semibold">{invitation.team.name}</span>
                    </h3>
                    <p className="mt-1 text-sm text-gray-600">
                      You've been invited by {invitation.invited_by.name} ({invitation.invited_by.email}) to join as a <span className="font-medium">{invitation.role}</span>.
                    </p>
                    <p className="mt-1 text-xs text-gray-500">
                      Invited on {new Date(invitation.created_at).toLocaleDateString()}
                    </p>
                  </div>
                  <div className="flex space-x-2">
                    <button
                      onClick={() => handleAcceptInvitation(invitation.pid)}
                      className="bg-green-500 hover:bg-green-600 text-white text-sm px-3 py-1 rounded"
                    >
                      Accept
                    </button>
                    <button
                      onClick={() => handleRejectInvitation(invitation.pid)}
                      className="bg-red-500 hover:bg-red-600 text-white text-sm px-3 py-1 rounded"
                    >
                      Reject
                    </button>
                  </div>
                </div>
              </li>
            ))}
          </ul>
        </div>
      )}
    </div>
  );
};

export default UserInvitations;
